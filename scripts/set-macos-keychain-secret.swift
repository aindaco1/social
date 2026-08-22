import Foundation
import Security

func fail(_ message: String, _ status: Int32) -> Never {
    FileHandle.standardError.write(Data("\(message)\n".utf8))
    exit(status)
}

guard CommandLine.arguments.count >= 4 else {
    fail("usage: keychain-helper <get|set> <service> <account> [label]", 64)
}

let operation = CommandLine.arguments[1]
let service = CommandLine.arguments[2]
let account = CommandLine.arguments[3]
let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: service,
    kSecAttrAccount as String: account,
]

if operation == "get" {
    var lookup = query
    lookup[kSecReturnData as String] = true
    lookup[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(lookup as CFDictionary, &result)

    guard status == errSecSuccess, let secret = result as? Data, !secret.isEmpty else {
        let message = SecCopyErrorMessageString(status, nil) as String? ?? "credential is empty"
        fail("Keychain read failed: \(message) (\(status))", Int32(status))
    }

    FileHandle.standardOutput.write(secret)
    exit(0)
}

guard operation == "set", CommandLine.arguments.count == 5 else {
    fail("usage: keychain-helper set <service> <account> <label>", 64)
}

let label = CommandLine.arguments[4]
var secret = FileHandle.standardInput.readDataToEndOfFile()

while secret.last == 0x0a || secret.last == 0x0d {
    secret.removeLast()
}

guard !secret.isEmpty else {
    fail("credential input is empty", 65)
}

let updates: [String: Any] = [
    kSecValueData as String: secret,
    kSecAttrLabel as String: label,
]
let existing = SecItemCopyMatching(query as CFDictionary, nil)
let status: OSStatus

if existing == errSecSuccess {
    status = SecItemUpdate(query as CFDictionary, updates as CFDictionary)
} else if existing == errSecItemNotFound {
    var addition = query
    updates.forEach { addition[$0.key] = $0.value }
    status = SecItemAdd(addition as CFDictionary, nil)
} else {
    status = existing
}

secret.resetBytes(in: 0..<secret.count)

guard status == errSecSuccess else {
    let message = SecCopyErrorMessageString(status, nil) as String? ?? "unknown Keychain error"
    fail("Keychain update failed: \(message) (\(status))", Int32(status))
}
