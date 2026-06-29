// Lode multi-thread spike (Swift, Dispatch.concurrentPerform). Models RFC-0012
// per-stream isolation: each "stream" mines its own pre-masked workload on its OWN
// parse tree, in parallel. Throughput is aggregate. Run threads=1 and threads=N.
// Build: swiftc -O mining_spike_mt.swift -o mining_spike_mt
// Run:   ./mining_spike_mt [N_total] [threads]   (defaults 8_000_000, 1)
import Dispatch

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
    var u = Array(s.utf8); if u.first == 45 { u.removeFirst() }; if u.isEmpty { return false }
    var any = false; for b in u { if isDigit(b) { any = true } else if b != 46 { return false } }; return any
}
func isIP(_ s: String) -> Bool {
    let p = s.split(separator: ".", omittingEmptySubsequences: false)
    if p.count != 4 { return false }; for x in p { if x.isEmpty || !x.utf8.allSatisfy(isDigit) { return false } }; return true
}
func isUUID(_ s: String) -> Bool {
    let u = Array(s.utf8); if u.count != 36 { return false }
    for (i, b) in u.enumerated() { if i == 8 || i == 13 || i == 18 || i == 23 { if b != 45 { return false } } else if !isHexB(b) { return false } }; return true
}
func isTS(_ s: String) -> Bool { var c = 0, any = false; for b in s.utf8 { if b == 58 { c += 1 } else if isDigit(b) { any = true } }; return c >= 2 && any }
func isPath(_ s: String) -> Bool { s.utf8.count > 1 && s.contains("/") }
func mask(_ tok: String) -> String {
    if isUUID(tok) { return "<UUID>" }; if isIP(tok) { return "<IP>" }; if isTS(tok) { return "<TS>" }
    if isPath(tok) { return "<PATH>" }; if isHex(tok) { return "<HEX>" }; if isNum(tok) { return "<NUM>" }; return tok
}
func hex16(_ x: UInt64) -> String { let s = String(x, radix: 16); return String(repeating: "0", count: max(0, 16 - s.count)) + s }
@inline(__always) func routeKey(_ ids: [Int], _ end: Int, _ len: Int) -> UInt64 {
    var h: UInt64 = 1469598103934665603 ^ UInt64(len); var k = 0
    while k < end { h = (h &* 1099511628211) ^ UInt64(bitPattern: Int64(ids[k])); k += 1 }; return h
}
@inline(__always) func simIds(_ a: [Int], _ b: [Int]) -> Double {
    if a.count != b.count { return 0 }; if a.isEmpty { return 1 }
    var m = 0; for i in 0..<a.count where a[i] == b[i] { m += 1 }; return Double(m) / Double(a.count)
}

func mine(_ masked: [[Int]], _ lines: Int, _ wildcard: Int, _ idToStr: [String]) -> (Int, UInt64) {
    let d = 4, st = 0.5
    var templates: [(ids: [Int], count: UInt64)] = []; templates.reserveCapacity(64)
    var index: [UInt64: [Int]] = [:]; index.reserveCapacity(64)
    for i in 0..<lines {
        let ids = masked[i % masked.count]
        let len = ids.count; let pref = min(d, len); let key = routeKey(ids, pref, len)
        var best = -1; var bestSim = st
        if let cands = index[key] { for ti in cands { let s = simIds(templates[ti].ids, ids); if s >= bestSim { bestSim = s; best = ti } } }
        if best >= 0 {
            let tlen = templates[best].ids.count
            for j in 0..<tlen where templates[best].ids[j] != ids[j] && templates[best].ids[j] != wildcard { templates[best].ids[j] = wildcard }
            templates[best].count += 1
        } else { let id = templates.count; templates.append((ids, 1)); index[key, default: []].append(id) }
    }
    var pats = templates.map { t in t.ids.map { idToStr[$0] }.joined(separator: " ") }; pats.sort()
    var h: UInt64 = 0xcbf29ce484222325
    for s in pats { for b in s.utf8 { h ^= UInt64(b); h = h &* 0x100000001b3 }; h ^= 10; h = h &* 0x100000001b3 }
    return (templates.count, h)
}

let args = CommandLine.arguments
let n = args.count > 1 ? (Int(args[1]) ?? 8_000_000) : 8_000_000
let threads = args.count > 2 ? (Int(args[2]) ?? 1) : 1

var interner: [String: Int] = [:]; var idToStr: [String] = []
func intern(_ s: String) -> Int { if let id = interner[s] { return id }; let id = idToStr.count; interner[s] = id; idToStr.append(s); return id }
let WILDCARD = intern("<*>")
let maskedIds: [[Int]] = SAMPLE.map { $0.split(separator: " ").map { intern(mask(String($0))) } }

let per = n / threads
var results = [(Int, UInt64)](repeating: (0, 0), count: threads)
let clock = ContinuousClock(); let t0 = clock.now
results.withUnsafeMutableBufferPointer { buf in
    DispatchQueue.concurrentPerform(iterations: threads) { idx in
        buf[idx] = mine(maskedIds, per, WILDCARD, idToStr)
    }
}
let el = clock.now - t0
let secs = Double(el.components.seconds) + Double(el.components.attoseconds) / 1e18
let total = per * threads
let hash = results[0].1
let allSame = results.allSatisfy { $0.1 == hash }
let thru = Int(Double(total) / secs + 0.5)
print("threads=\(threads) total_lines=\(total) time=\(Double(Int(secs * 1000 + 0.5)) / 1000)s throughput=\(thru) l/s templates/thread=\(results[0].0) determinism_ok=\(allSame) hash=\(hex16(hash))")
