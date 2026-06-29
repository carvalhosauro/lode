// Lode mining-core spike (Swift, OPTIMIZED). Same algorithm + same template_set_hash
// as mining_spike.swift, but the hot loop is allocation-free:
//   - tokens are interned to Int ids ONCE (outside the timed loop)
//   - routing key is an integer hash (no per-line String build)
//   - similarity compares Int ids, not Strings
// Build: swiftc -O mining_spike_opt.swift -o mining_spike_opt
// Run:   ./mining_spike_opt [N]      (default 1_000_000)

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

func isHex(_ s: String) -> Bool { let u = Array(s.utf8); return u.count >= 8 && u.allSatisfy(isHexB) }
func isNum(_ s: String) -> Bool {
    var u = Array(s.utf8)
    if u.first == 45 { u.removeFirst() }
    if u.isEmpty { return false }
    var anyDigit = false
    for b in u { if isDigit(b) { anyDigit = true } else if b != 46 { return false } }
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

let d = 4
let st = 0.5
let n = CommandLine.arguments.count > 1 ? (Int(CommandLine.arguments[1]) ?? 1_000_000) : 1_000_000

// --- Interning (done ONCE, outside the timed loop) ---
var interner: [String: Int] = [:]
var idToStr: [String] = []
func intern(_ s: String) -> Int {
    if let id = interner[s] { return id }
    let id = idToStr.count; interner[s] = id; idToStr.append(s); return id
}
let WILDCARD = intern("<*>")   // reserve a stable id for the wildcard

let maskedIds: [[Int]] = SAMPLE.map { line in line.split(separator: " ").map { intern(mask(String($0))) } }

struct Template { var ids: [Int]; var count: UInt64 }
var templates: [Template] = []; templates.reserveCapacity(256)
var index: [UInt64: [Int]] = [:]; index.reserveCapacity(256)

@inline(__always)
func routeKey(_ ids: [Int], _ end: Int, _ len: Int) -> UInt64 {
    var h: UInt64 = 1469598103934665603 ^ UInt64(len)
    var k = 0
    while k < end { h = (h &* 1099511628211) ^ UInt64(bitPattern: Int64(ids[k])); k += 1 }
    return h
}
@inline(__always)
func similarityIds(_ a: [Int], _ b: [Int]) -> Double {
    if a.count != b.count { return 0 }
    if a.isEmpty { return 1 }
    var m = 0
    for i in 0..<a.count where a[i] == b[i] { m += 1 }
    return Double(m) / Double(a.count)
}

let clock = ContinuousClock()
let startT = clock.now
for i in 0..<n {
    let ids = maskedIds[i % maskedIds.count]
    let len = ids.count
    let prefEnd = min(d, len)
    let key = routeKey(ids, prefEnd, len)

    var best = -1
    var bestSim = st
    if let cands = index[key] {
        for ti in cands {
            let s = similarityIds(templates[ti].ids, ids)
            if s >= bestSim { bestSim = s; best = ti }
        }
    }
    if best >= 0 {
        let tlen = templates[best].ids.count
        for j in 0..<tlen where templates[best].ids[j] != ids[j] && templates[best].ids[j] != WILDCARD {
            templates[best].ids[j] = WILDCARD
        }
        templates[best].count += 1
    } else {
        let id = templates.count
        templates.append(Template(ids: ids, count: 1))
        index[key, default: []].append(id)
    }
}
let elapsed = clock.now - startT
let secs = Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18

func pad(_ x: UInt64, _ w: Int) -> String { let s = String(x); return String(repeating: " ", count: max(0, w - s.count)) + s }
func hex16(_ x: UInt64) -> String { let s = String(x, radix: 16); return String(repeating: "0", count: max(0, 16 - s.count)) + s }

// rebuild patterns as strings via idToStr — identical strings => identical hash to the naive spike
var patterns = templates.map { t in t.ids.map { idToStr[$0] }.joined(separator: " ") }
patterns.sort()
var h: UInt64 = 0xcbf29ce484222325
for s in patterns {
    for b in s.utf8 { h ^= UInt64(b); h = h &* 0x100000001b3 }
    h ^= UInt64(10); h = h &* 0x100000001b3
}

var out = templates.map { t in "\(pad(t.count, 10))  \(t.ids.map { idToStr[$0] }.joined(separator: " "))" }
out.sort()
print("=== templates (\(templates.count)) ===")
for l in out { print(l) }
print("---")
let thru = Int(Double(n) / secs + 0.5)
let ms = Int(secs * 1000 + 0.5)
print("lines=\(n) time=\(Double(ms) / 1000)s throughput=\(thru) lines/sec")
print("template_set_hash=\(hex16(h))")
