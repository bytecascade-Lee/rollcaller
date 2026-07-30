use crate::common::entity::student::Student;
use crate::common::entity::student::StudentTable;
use crate::common::enums::student::{StudentBatchCreateResult, StudentSingleCreateResult, StudentSingleUpdate};
use crate::repo::student_repo;
use anyhow::anyhow;
use rbatis::executor::Executor;
use rbatis::rbatis_codegen::ops::AsProxy;
use rbatis::RBatis;
use rbs::value;
use serde::de::Error;
use serde::Deserialize;
use sha2::digest::typenum::Len;
use std::cmp::PartialEq;
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// 创建单个学生
///
/// 该函数会先查询是否存在与给定学号相同记录，然后根据记录状态和用户决策执行不同操作
///
/// # 参数
///
/// * `rb` - 数据库连接池引用
/// * `student_no` - 学生学号
/// * `name` - 学生姓名
/// * `decision` - 用户对冲突处理的决策，`Some(true)` 表示允许覆写，`Some(false)` 表示禁止覆写，`None` 表示未明确决策
///
/// # 执行
///
/// * 插入新的
/// * 恢复已被删除的
/// * 覆写已删除的
/// * 保持已存在的
/// * 与活跃的产生冲突
/// * 与删除的产生冲突
///
/// # 返回
///
/// 返回 `anyhow::Result<StudentSingleCreate>`，其中 `StudentSingleCreate` 是状态枚举，携带 `StudentTable` 即操作后的学生数据。
///
/// 可能返回的状态枚举包括：
///
/// * [`StudentSingleCreateResult::Insert`] - 学号不存在，成功插入新学生记录
/// * [`StudentSingleCreateResult::Restore`] - 学号曾存在但已删除，且姓名相同，执行恢复操作
/// * [`StudentSingleCreateResult::Override`] - 学号曾存在且已删除，但姓名不同，用户同意覆写
/// * [`StudentSingleCreateResult::ActiveExists`] - 学号已存在且为活跃状态（未删除），操作中止
/// * [`StudentSingleCreateResult::Retain`] - 学号曾存在且已删除，但姓名不同，用户拒绝覆写，保留原记录
/// * [`StudentSingleCreateResult::Conflict`] - 学号曾存在且已删除，但姓名不同，用户未明确决策，操作中止
/// * [`Err`] - 数据库操作或事务执行过程中发生意外错误。
///
/// # 事务
///
/// 函数内部开启数据库事务，根据操作结果自动提交或回滚。
pub async fn create(
    rb: &RBatis,
    student_no: String,
    name: String,
    decision: Option<bool>,
) -> anyhow::Result<StudentSingleCreateResult> {
    // 1. 开启事务，锁库
    let mut tx = rb.acquire_begin().await?;
    // 2. 查询所有与给定学号相同的学生（理论上长度为1）
    let all_matched = StudentTable::select_by_map(&mut tx, value! {"student_no": &student_no}).await?;
    let length = all_matched.len();
    if length > 1 {
        return Err(anyhow!("学号 {} 有多余一条记录！", length));
    }
    let matched_student = all_matched.into_iter().next();
    // 3. 解包 Option
    match matched_student {
        //* 3.1 存在该学号的学生
        Some(exist) => {
            //* 3.1.1 未删除
            if !exist.is_deleted {
                warn!("Student:[{}] already exists.", exist.student_no);
                tx.rollback().await?;
                return Ok(StudentSingleCreateResult::ActiveExists(exist.clone()));
            }
            //* 3.1.2 已删除且其他字段相同
            // 执行恢复
            if exist.name == name {
                student_repo::restore(&mut tx, vec![exist.id]).await?;
                tx.commit().await?;
                //~ 此处的查询可不必，只需要将exist的is_deleted设为false，deleted_at设为None即可
                let vec = StudentTable::select_by_map(rb, value! {"id": exist.id}).await?;
                return Ok(StudentSingleCreateResult::Restore(vec.into_iter().next().unwrap()));
            }
            //* 已删除但其他字段不同
            match decision {
                Some(decision) => {
                    if decision {
                        //* 3.1.3 用户同意覆写
                        student_repo::update_name(&mut tx, exist.id, &name).await?;
                        student_repo::restore(&mut tx, vec![exist.id]).await?;
                        tx.commit().await?;
                        //~ 此处的查询可不必，只需要将exist的name设为新值即可
                        let vec = StudentTable::select_by_map(rb, value! {"id": exist.id}).await?;
                        Ok(StudentSingleCreateResult::Override(vec.into_iter().next().unwrap()))
                    } else {
                        //* 3.1.4 用户禁止覆写
                        tx.rollback().await?;
                        Ok(StudentSingleCreateResult::Retain(exist.clone()))
                    }
                }
                //* 3.1.5 未明确传递是否覆写
                None => {
                    warn!(
                        "The student:[{}] already exists, but we don't know if we want to cover TA.",
                        student_no
                    );
                    tx.rollback().await?;
                    Ok(StudentSingleCreateResult::Conflict(exist.clone()))
                }
            }
        }

        //* 3.2 不存在该学号的学生
        None => {
            let result = Student::insert(&mut tx, &Student::new(&student_no, &name)).await?;
            tx.commit().await?;
            let inserted_id = result.last_insert_id.as_i64().unwrap();
            let vec = StudentTable::select_by_map(rb, value! {"id": inserted_id}).await?;
            Ok(StudentSingleCreateResult::Insert(vec.into_iter().next().unwrap()))
        }
    }
}

/// 批量创建学生记录
///
/// 自动处理与已有数据的冲突、软删除恢复及姓名覆盖决策。
///
/// # 参数
///
/// * `rb` - 数据库连接池引用，用于开启事务及事务提交后的独立查询。
/// * `students` - 待创建的学生列表。每个学生的学号必须外部确保已去除不可见字符，函数内部不做去空格处理。
/// * `decisions` - 当遇到已删除但姓名不同的记录时，由调用方提供的决策映射：
///   - 键为学号 (`student_no`)，
///   - 值为 `true` 表示用新姓名覆盖并恢复该记录，
///   - 值为 `false` 表示丢弃该新记录，保留已删除状态不作任何操作。
///
/// # 执行
///
/// 1. **输入去重检查**
///    提取所有学号并排序，若发现重复学号，直接返回 `Repeat`，不接触数据库。
///
/// 2. **事务开始与数据库查询**
///    开启事务（可能会锁库），查询数据库中所有与给定学号匹配的学生记录（包括软删除记录）。
///
/// 3. **无冲突快速路径**
///    若数据库中不存在任何这些学号的记录，则在事务内批量插入全部学生，提交后再次查询这些记录并返回 `Insert`。
///
/// 4. **逐条分类处理**
///    对每个输入的学生，按已存在记录的状态分入五类：
///    - **新记录**（数据库中无此学号） → 待插入列表
///    - **活跃冲突**（数据库中有，且未被删除） → 冲突列表
///    - **自动恢复**（数据库中有，已删除，且姓名相同） → 待恢复列表
///    - **待决策**（数据库中有，已删除，但姓名不同） → 暂存待决策列表
///
/// 5. **冲突检查**
///    若存在任何活跃冲突，回滚事务并返回 `Conflict`。
///
/// 6. **决策解析**
///    遍历待决策项，查询 `decisions`：
///    - 若映射中无此学号 → 标记为需要决策
///    - 若值为 `true` → 标记为覆盖（更新姓名并恢复）
///    - 若值为 `false` → 丢弃
///
///    若存在任何需要决策的记录，回滚事务并返回 `NeedDecide`。
///
/// 7. **事务内写操作**
///    所有检查通过后，在同一事务内顺序执行：
///    - 批量插入待插入的学生
///    - 批量恢复待恢复的记录（仅将 `is_deleted` 置为 0）
///    - 依次处理覆盖项：先用新姓名更新数据库记录，再恢复该记录（置 `is_deleted` 为 0）
///
/// 8. **事务提交**
///    成功执行所有写操作后提交事务。
///
/// 9. **结果查询与返回**
///    事务结束后，使用非事务连接 `rb` 分别查询最终状态的新插入、已恢复及已覆盖记录，合并为 `Upsert` 结果返回。
///
/// # 返回
///
/// 返回 `anyhow::Result<StudentSingleCreate>`，其中 `StudentSingleCreate` 是状态枚举，携带 `StudentTable` 即操作后的学生数据。
///
/// 可能返回的枚举包括：
///
/// * [`StudentBatchCreateResult::Insert`] - 数据库中原本无任何冲突，全部为新记录直接插入成功。
/// * [`StudentBatchCreateResult::Upsert`] - 部分插入、部分恢复或部分覆盖，所有受影响的最新记录列表。
/// * [`StudentBatchCreateResult::DuplicateInput`] - 输入学生中存在重复的学号，操作未执行。
/// * [`StudentBatchCreateResult::DecisionRequired`] - 存在已删除但姓名不同的记录，且调用方未提供决策，需要进一步交互。
/// * [`StudentBatchCreateResult::Conflict`] - 存在活跃的未删除冲突记录，操作未执行。
/// * [`Err`] - 数据库操作或事务执行过程中发生意外错误。
///
/// # 事务
///
/// 整个数据库写操作被包裹在一个由 `rb.acquire_begin()` 开启的事务中。
/// - 任何非 `Upsert` 或 `Insert` 的返回路径（`Repeat`、`Conflict`、`NeedDecide`）以及数据库错误都会显式调用 `tx.rollback()`。
/// - 仅在成功写入全部数据后执行 `tx.commit()`。
/// - 返回 `Upsert` 时使用的查询走普通连接，因此能读取到刚提交的数据，同时不受事务隔离影响。
pub async fn batch_create(
    rb: &RBatis,
    students: Vec<Student>,
    decisions: HashMap<String, bool>,
) -> anyhow::Result<StudentBatchCreateResult> {
    //? 可能存在的bug：学号的首位包含空白字符

    // 准备工作：提取学号
    let student_nos: Vec<String> = students.iter().map(|s| s.student_no.clone()).collect();

    // 检查学号是否有重复的
    let mut sorted = student_nos.clone();
    sorted.sort();
    let mut duplicates = Vec::new();
    let mut i = 0;
    while i < sorted.len() - 1 {
        if sorted[i] == sorted[i + 1] {
            duplicates.push(sorted[i].clone());
            // 跳过所有相同的元素
            while i < sorted.len() - 1 && sorted[i] == sorted[i + 1] {
                i += 1;
            }
        }
        i += 1;
    }
    if !duplicates.is_empty() {
        return Ok(StudentBatchCreateResult::DuplicateInput(duplicates));
    }

    // 开启事务，锁库
    let mut tx = rb.acquire_begin().await?;

    // 查询所有与给定学号相同的学生
    let existing = StudentTable::select_by_map(&mut tx, value! {"student_no": &student_nos}).await?;

    // 如果没有冲突，直接全部插入
    if existing.is_empty() {
        Student::insert_batch(&mut tx, &*students, students.len() as u64).await?;
        tx.commit().await?;
        let inserted_students = StudentTable::select_by_map(rb, value! {"student_no": student_nos}).await?;
        return Ok(StudentBatchCreateResult::Insert(inserted_students));
    }

    // 分类处理
    let mut to_insert = Vec::new();
    let mut to_restore = Vec::new();
    let mut to_override = Vec::new();
    let mut to_decide = Vec::new();
    let mut conflicts = Vec::new();
    //. 克隆一下，后面仍旧需要使用
    for student in students.clone() {
        match existing.iter().find(|s| s.student_no == student.student_no) {
            None => {
                to_insert.push(student);
            }
            Some(exist) => {
                if !exist.is_deleted {
                    // 活跃记录，冲突，只能放弃
                    conflicts.push(exist.clone())
                } else if student.name == exist.name {
                    // 已删除且姓名相同，自动恢复
                    to_restore.push(exist);
                } else {
                    // 已删除但姓名不同，需要决策覆盖
                    to_decide.push((student, exist.clone()));
                }
            }
        }
    }

    // 如果有冲突，直接返回
    if !conflicts.is_empty() {
        tx.rollback().await?;
        return Ok(StudentBatchCreateResult::Conflict(conflicts));
    }

    // 如果有未决策的，也返回
    let mut require_decide = Vec::new();
    for (student, st) in to_decide {
        match decisions.get(&st.student_no) {
            None => require_decide.push(st),
            Some(decision) => {
                if *decision {
                    to_override.push((student, st));
                } else {
                    // 丢弃该值
                }
            }
        }
    }
    if !require_decide.is_empty() {
        tx.rollback().await?;
        return Ok(StudentBatchCreateResult::DecisionRequired(require_decide));
    }

    // 执行插入操作
    let inserted_student_nos: Vec<String> = to_insert.iter().map(|s| s.student_no.clone()).collect();
    if !to_insert.is_empty() {
        Student::insert_batch(&mut tx, &*to_insert, to_insert.len() as u64).await?;
    }

    // 执行恢复操作
    let restored_ids: Vec<i64> = to_restore.iter().map(|s| s.id).collect();
    if !to_restore.is_empty() {
        student_repo::restore(&mut tx, restored_ids.clone()).await?;
    }

    // 执行覆写事务
    let override_ids: Vec<i64> = to_override.iter().map(|(_, st)| st.id).collect();
    if !to_override.is_empty() {
        for (s, st) in to_override {
            student_repo::update_name(&mut tx, st.id, &*s.name).await?;
        }
        student_repo::restore(&mut tx, override_ids.clone()).await?;
    }

    // 提交事务
    tx.commit().await?;

    let inserted_students = StudentTable::select_by_map(rb, value! {"student_no": inserted_student_nos}).await?;
    let restored_students = StudentTable::select_by_map(rb, value! {"id": restored_ids}).await?;
    let override_students = StudentTable::select_by_map(rb, value! {"id": override_ids}).await?;

    let mut result = Vec::with_capacity(inserted_students.len() + restored_students.len() + override_students.len());
    result.extend(inserted_students);
    result.extend(restored_students);
    result.extend(override_students);

    Ok(StudentBatchCreateResult::Upsert(result))
}

/// 查询所有学生，不包括已删除的
pub async fn get_all(rb: &dyn Executor) -> anyhow::Result<Vec<StudentTable>> {
    Ok(StudentTable::select_by_map(rb, value! {"is_deleted": 0}).await?)
}

/// 按 ID 列表查询
pub async fn get_by_ids(rb: &dyn Executor, ids: Vec<i64>) -> anyhow::Result<Vec<StudentTable>> {
    Ok(StudentTable::select_by_map(rb, value! {"id": ids}).await?)
}

/// 按学号列表查询
pub async fn get_by_student_nos(rb: &dyn Executor, student_nos: Vec<String>) -> anyhow::Result<Vec<StudentTable>> {
    Ok(StudentTable::select_by_map(rb, value! {"student_no": student_nos}).await?)
}

/// 更新学生
pub async fn update(rb: &RBatis, student: Student) -> anyhow::Result<StudentSingleUpdate> {
    if student.id == None {
        return Err(anyhow!("id不能为空"));
    }
    let mut tx = rb.acquire_begin().await?;
    let existing = StudentTable::select_by_map(&mut tx, value! {"student_no": &student.student_no}).await?;
    if existing.len() > 0 {
        tx.rollback().await?;
        return Ok(StudentSingleUpdate::Conflict(existing.into_iter().next().unwrap()));
    }

    // 此处的id必定不为None
    Student::update_by_map(&mut tx, &student, value! {"id": student.id}).await?;
    tx.commit().await?;
    Ok(StudentSingleUpdate::Update(
        StudentTable::select_by_map(rb, value! {"id": student.id})
            .await?
            .into_iter()
            .next()
            .unwrap(),
    ))
}

/// 删除学生
pub async fn delete(rb: &RBatis, ids: Vec<i64>) -> anyhow::Result<()> {
    let mut tx = rb.acquire_begin().await?;
    student_repo::delete(&mut tx, ids).await?;
    tx.commit().await?;
    Ok(())
}

/// 恢复学生
pub async fn restore(rb: &RBatis, ids: Vec<i64>) -> anyhow::Result<()> {
    let mut tx = rb.acquire_begin().await?;
    student_repo::restore(&mut tx, ids).await?;
    tx.commit().await?;
    Ok(())
}
