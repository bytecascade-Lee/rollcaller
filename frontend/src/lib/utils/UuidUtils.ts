export function uuid() {
  return crypto.randomUUID().replace(/-/g, "").substring(0, 8)
}
