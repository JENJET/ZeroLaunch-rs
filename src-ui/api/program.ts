export interface ProgramDisplayInfo {
    name: string
    path: string
    program_guid: string  // ✅ 使用字符串避免JavaScript数字精度丢失（u64超出2^53-1）
    icon_request_json: string
    is_builtin: boolean
}
