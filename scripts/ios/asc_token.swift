// Prints a 20-minute App Store Connect API token (an ES256 JWT) for the team
// key in ~/.appstoreconnect/private_keys/AuthKey_<KEY_ID>.p8.
//
// Usage: ASC_KEY_ID=... ASC_ISSUER_ID=... swift scripts/ios/asc_token.swift
// Then: curl -H "Authorization: Bearer $(swift scripts/ios/asc_token.swift)" \
//         https://api.appstoreconnect.apple.com/v1/apps
// The key never leaves this Mac; only the short-lived token is sent.
import CryptoKit
import Foundation

let env = ProcessInfo.processInfo.environment
guard let keyID = env["ASC_KEY_ID"], let issuer = env["ASC_ISSUER_ID"] else {
    FileHandle.standardError.write("Set ASC_KEY_ID and ASC_ISSUER_ID.\n".data(using: .utf8)!)
    exit(1)
}
// ASC_KEY_PATH overrides where the .p8 is read from.
let path = env["ASC_KEY_PATH"].map { URL(fileURLWithPath: $0) } ?? FileManager.default.homeDirectoryForCurrentUser
    .appendingPathComponent(".appstoreconnect/private_keys/AuthKey_\(keyID).p8")
let pem = try String(contentsOf: path, encoding: .utf8)
let key = try P256.Signing.PrivateKey(pemRepresentation: pem)

func base64url(_ data: Data) -> String {
    data.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
}

let now = Int(Date().timeIntervalSince1970)
let header = try JSONSerialization.data(withJSONObject: ["alg": "ES256", "kid": keyID, "typ": "JWT"])
let claims = try JSONSerialization.data(withJSONObject: ["iss": issuer, "iat": now, "exp": now + 1200, "aud": "appstoreconnect-v1"])
let input = base64url(header) + "." + base64url(claims)
let signature = try key.signature(for: Data(input.utf8))
print(input + "." + base64url(signature.rawRepresentation))
