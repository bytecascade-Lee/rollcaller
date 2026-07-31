use crate::common::entity::student::StudentTable;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 学生单个创建操作的结果枚举。
///
/// 该枚举描述了针对单个学生记录执行创建操作时，系统根据数据库当前状态
/// 和调用方提供的决策参数，所给出的具体处理结果。
///
/// 每个变体都包含一个 [`StudentTable`] 对象，代表操作生效后数据库中的最终记录。
#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum StudentSingleCreateResult {
    /// 数据库中无现存记录，新学生信息直接插入成功。
    ///
    /// 该变体表示目标学生在数据库中完全不存在，系统执行了纯插入操作，
    /// 并返回新插入的完整记录。
    Insert(StudentTable),

    /// 已删除记录内容与请求完全一致，已自动恢复。
    ///
    /// 该变体表示数据库中存在一条已被软删除的学生记录，且其所有业务字段
    /// 与本次请求完全一致。系统自动将其恢复为活跃状态，并返回恢复后的记录。
    Restore(StudentTable),

    /// 已删除记录内容与请求不一致，已使用新值覆盖。
    ///
    /// 该变体表示数据库中存在一条已被软删除的学生记录，但其业务字段与本次请求
    /// 不一致。调用方已明确允许覆盖，系统使用新数据覆盖原记录，并返回更新后的记录。
    Override(StudentTable),

    /// 已删除记录内容与请求不一致，已保留原值不变。
    ///
    /// 该变体表示数据库中存在一条已被软删除的学生记录，但其业务字段与本次请求
    /// 不一致。调用方已明确禁止覆盖，系统保留原记录不变（仍为软删除状态），
    /// 并返回该保留的原有记录。
    Retain(StudentTable),

    /// 存在活跃记录，操作被拒绝。
    ///
    /// 该变体表示数据库中存在一条状态为活跃（未删除）的学生记录，
    /// 为避免数据冲突，系统拒绝执行本次创建操作。调用方需先处理现有活跃记录，
    /// 例如将其删除或合并，方可重试。返回的 [`StudentTable`] 为当前活跃记录。
    ActiveExists(StudentTable),

    /// 已删除记录内容不一致，且调用方未明确传递决策，操作暂停。
    ///
    /// 该变体表示数据库中存在一条已被软删除的学生记录，其业务字段与本次请求
    /// 存在差异，且调用方未明确指示应覆盖还是保留。系统无法自主决定，
    /// 需调用方补充决策参数后重试。返回的 [`StudentTable`] 为现有已删除记录。
    Conflict(StudentTable),
}

/// 学生批量创建操作的结果枚举。
///
/// 该枚举描述了针对多个学生记录执行批量创建操作时，系统处理后的整体结果。
/// 批量操作采用“失败即回滚”的策略，保证数据的一致性。
///
/// 各变体携带的具体数据不同，调用方应根据变体类型进行相应处理：
/// - 成功类变体（[`Insert`](Self::Insert)、[`Upsert`](Self::Upsert)）携带最终记录列表；
/// - 失败类变体携带错误详情或待处理的记录列表，供调用方排查或重试。
#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum StudentBatchCreateResult {
    /// 所有记录均为新记录，全部插入成功。
    ///
    /// 该变体表示批量中的所有学生记录在数据库中均不存在（包括无任何已删除记录），
    /// 系统全部执行插入操作。返回的 [`Vec<StudentTable>`] 包含所有成功插入的记录。
    Insert(Vec<StudentTable>),

    /// 所有记录均已成功处理，包含插入、恢复或覆盖操作。
    ///
    /// 该变体表示批量操作已完成，所有记录均被成功写入数据库。
    /// 具体到单条记录的处理方式可能为插入、恢复或覆盖，返回的 [`Vec<StudentTable>`]
    /// 包含所有记录处理后的最终生效版本。
    Upsert(Vec<StudentTable>),

    /// 输入数据中存在重复的学号，操作未执行。
    ///
    /// 该变体表示调用方提供的批量数据中，存在两条或以上拥有相同学号的学生记录。
    /// 系统拒绝执行本次批量操作，返回的 [`Vec<String>`] 为所有重复的学号列表，
    /// 调用方需去重后重新发起请求。
    DuplicateInput(Vec<String>),

    /// 存在已删除但字段不一致的记录，且调用方未提供决策，操作暂停。
    ///
    /// 该变体表示批量中存在部分学生记录在数据库中已被软删除，但业务字段与请求不一致，
    /// 且调用方未传递覆盖/保留决策。系统无法自动处理这部分记录，
    /// 返回的 [`Vec<StudentTable>`] 为所有涉及冲突的现有记录。
    /// 调用方需为每条冲突记录补充决策后重试。
    DecisionRequired(Vec<StudentTable>),

    /// 存在活跃的未删除冲突记录，操作未执行。
    ///
    /// 该变体表示批量中存在部分学生记录在数据库中处于活跃（未删除）状态，
    /// 与本次请求产生冲突。系统拒绝执行本次批量操作，返回的 [`Vec<StudentTable>`]
    /// 为所有冲突的现有活跃记录。调用方需先处理这些活跃记录（如删除）
    /// 方可重新发起请求。
    Conflict(Vec<StudentTable>),
}

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[serde(tag = "type", content = "data")]
#[ts(export)]
pub enum StudentSingleUpdate {
    Update(StudentTable),
    Conflict(StudentTable),
}

