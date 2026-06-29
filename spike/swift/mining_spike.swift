// Lode mining-core spike (Swift). Throwaway: language comparison + RFC-0003 sanity.
// Pure Swift stdlib, no Foundation (editxr zero-dep spirit).
// Build: swiftc -O mining_spike.swift -o mining_spike
// Run:   ./mining_spike [N_lines]   (default 1_000_000)
// Or:    swift mining_spike.swift 1000000
//
// The template_set_hash MUST equal the Rust spike's hash on the same sample:
// that cross-validates both implement the identical algorithm.

let SAMPLE: [String] = [
    "127.0.0.1 - - [10/Oct/2024:13:55:36] \"GET /api/users/12 HTTP/1.1\" 200 1534",
    "127.0.0.1 - - [10/Oct/2024:13:55:37] \"GET /api/users/47 HTTP/1.1\" 200 1422",
    "192.168.1.5 - - [10/Oct/2024:13:55:38] \"POST /api/login HTTP/1.1\" 401 88",
    "INFO 2024-10-10 13:55:36 user 12 logged in from 10.0.0.3",
    "INFO 2024-10-10 13:55:37 user 47 logged in from 10.0.0.9",
    "ERROR 2024-10-10 13:55:40 db connection failed after 3000 ms id 550e8400-e29b-41d4-a716-446655440000",
    "WARN 2024-10-10 13:55:41 cache miss for key a1b2c3d4e5f6a7b8",
    "GET /static/app.css 200",
]

@inline(__always) func isDigit(_ b: UInt8) -> Bool { b >= 48 && b <= 57 }
@inline(__always) func isHexB(_ b: UInt8) -> Bool { isDigit(b) || ((b | 0x20) >= 97 && (b | 0x20) <= 102) }

func isHex(_ s: String) -> Bool {
    let u = Array(s.utf8); return u.count >= 8 && u.allSatisfy(isHexB)
}
func isNum(_ s: String) -> Bool {
    var u = Array(s.utf8)
    if u.first == 45 { u.removeFirst() }            // leading '-'
    if u.isEmpty { return false }
    var anyDigit = false
    for b in u { if isDigit(b) { anyDigit = true } else if b != 46 { return false } } // '.' == 46
    return anyDigit
}
func isIP(_ s: String) -> Bool {
    let parts = s.split(separator: ".", omittingEmptySubsequences: false)
    if parts.count != 4 { return false }
    for p in parts { if p.isEmpty || !p.utf8.allSatisfy(isDigit) { return false } }
    return true
}
func isUUID(_ s: String) -> Bool {
    let u = Array(s.utf8)
    if u.count != 36 { return false }
    for (i, b) in u.enumerated() {
        if i == 8 || i == 13 || i == 18 || i == 23 { if b != 45 { return false } }
        else if !isHexB(b) { return false }
    }
    return true
}
func isTS(_ s: String) -> Bool {
    var colons = 0; var anyDigit = false
    for b in s.utf8 { if b == 58 { colons += 1 } else if isDigit(b) { anyDigit = true } }
    return colons >= 2 && anyDigit
}
func isPath(_ s: String) -> Bool { s.utf8.count > 1 && s.contains("/") }

func mask(_ tok: String) -> String {
    if isUUID(tok) { return "<UUID>" }
    if isIP(tok)   { return "<IP>" }
    if isTS(tok)   { return "<TS>" }
    if isPath(tok) { return "<PATH>" }
    if isHex(tok)  { return "<HEX>" }
    if isNum(tok)  { return "<NUM>" }
    return tok
}

struct Template { var tokens: [String]; var count: UInt64 }

func similarity(_ a: [String], _ b: [String]) -> Double {
    if a.count != b.count { return 0.0 }
    if a.isEmpty { return 1.0 }
    var m = 0
    for i in 0..<a.count { if a[i] == b[i] { m += 1 } }
    return Double(m) / Double(a.count)
}

func pad(_ x: UInt64, _ w: Int) -> String {
    let s = String(x); return String(repeating: " ", count: max(0, w - s.count)) + s
}
func hex16(_ x: UInt64) -> String {
    let s = String(x, radix: 16); return String(repeating: "0", count: max(0, 16 - s.count)) + s
}

let d = 4
let st = 0.5
let n = CommandLine.arguments.count > 1 ? (Int(CommandLine.arguments[1]) ?? 1_000_000) : 1_000_000

let masked: [[String]] = SAMPLE.map { $0.split(separator: " ").map { mask(String($0)) } }

var templates: [Template] = []
var index: [String: [Int]] = [:]    // key = "len\u{1}prefix"

let clock = ContinuousClock()
let startT = clock.now
for i in 0..<n {
    let toks = masked[i % masked.count]
    let len = toks.count
    let prefEnd = min(d, len)
    let key = "\(len)\u{1}" + toks[0..<prefEnd].joined(separator: "\u{1}")

    var best: Int? = nil
    var bestSim = st
    if let cands = index[key] {
        for ti in cands {
            let s = similarity(templates[ti].tokens, toks)
            if s >= bestSim { bestSim = s; best = ti }
        }
    }
    if let ti = best {
        for j in 0..<len where templates[ti].tokens[j] != toks[j] && templates[ti].tokens[j] != "<*>" {
            templates[ti].tokens[j] = "<*>"
        }
        templates[ti].count += 1
    } else {
        let id = templates.count
        templates.append(Template(tokens: toks, count: 1))
        index[key, default: []].append(id)
    }
}
let elapsed = clock.now - startT
let secs = Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18

// Deterministic template-set hash (FNV-1a over sorted patterns, counts excluded) — must match Rust.
var patterns = templates.map { $0.tokens.joined(separator: " ") }
patterns.sort()
var h: UInt64 = 0xcbf29ce484222325
for s in patterns {
    for b in s.utf8 { h ^= UInt64(b); h = h &* 0x100000001b3 }
    h ^= UInt64(10); h = h &* 0x100000001b3
}

var out = templates.map { "\(pad($0.count, 10))  \($0.tokens.joined(separator: " "))" }
out.sort()
print("=== templates (\(templates.count)) ===")
for l in out { print(l) }
print("---")
let thru = Int(Double(n) / secs + 0.5)        // integer rounding; avoids libm round()
let ms = Int(secs * 1000 + 0.5)
print("lines=\(n) time=\(Double(ms) / 1000)s throughput=\(thru) lines/sec")
print("template_set_hash=\(hex16(h))")
