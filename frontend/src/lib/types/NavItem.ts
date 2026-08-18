export type NavItem = {
  id: string;
  title: string;
  parentId: string | null;
  children: NavItem[];
}
