import AppKit
import Foundation

// Draws the same artwork as native/assets/brand.svg and `brand_mark` in
// native/src/design.rs: the magnet from native/assets/magnet.svg in white on a
// gradient tile. Coordinates use the SVG's 1024-unit, y-down space.
let folder = "native/assets/Torrent.iconset"
try FileManager.default.createDirectory(atPath: folder, withIntermediateDirectories: true)

func color(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat, _ a: CGFloat = 1) -> NSColor {
    NSColor(calibratedRed: r / 255, green: g / 255, blue: b / 255, alpha: a)
}

// magnet.svg places the magnet at (252, 425) tilted 35°; the logo then centers
// the trace's (317, 353) on the tile at 1.28x.
let scaleUp: CGFloat = 1.28
let tilt = 35 * CGFloat.pi / 180
func trace(_ x: CGFloat, _ y: CGFloat) -> NSPoint {
    NSPoint(x: 512 + scaleUp * (x - 317), y: 512 + scaleUp * (y - 353))
}
func magnet(_ x: CGFloat, _ y: CGFloat) -> NSPoint {
    trace(252 + x * cos(tilt) - y * sin(tilt), 425 + x * sin(tilt) + y * cos(tilt))
}

for size in [16, 32, 64, 128, 256, 512, 1024] {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let transform = NSAffineTransform()
    transform.scale(by: CGFloat(size) / 1024)
    transform.translateX(by: 0, yBy: 1024)
    transform.scaleX(by: 1, yBy: -1)
    transform.concat()

    let tile = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 200, yRadius: 200)
    NSGraphicsContext.saveGraphicsState()
    tile.addClip()
    NSGradient(colors: [color(56, 189, 248), color(99, 102, 241), color(139, 92, 246)],
               atLocations: [0, 0.5, 1], colorSpace: .deviceRGB)!
        .draw(from: NSPoint(x: 64, y: 64), to: NSPoint(x: 960, y: 960),
              options: [.drawsBeforeStartingLocation, .drawsAfterEndingLocation])
    NSGradient(starting: color(255, 255, 255, 0.18), ending: color(255, 255, 255, 0))!
        .draw(from: NSPoint(x: 512, y: 64), to: NSPoint(x: 512, y: 560), options: [.drawsBeforeStartingLocation])
    NSGraphicsContext.restoreGraphicsState()

    NSColor.white.setFill()
    NSColor.white.setStroke()
    // Body: the U's centerline (radius 128) stroked 84 wide with flat ends.
    let body = NSBezierPath()
    body.move(to: magnet(-128, -95))
    for step in 0...48 {
        let angle = CGFloat.pi * (1 - CGFloat(step) / 48)
        body.line(to: magnet(128 * cos(angle), 128 * sin(angle)))
    }
    body.line(to: magnet(128, -95))
    body.lineWidth = 84 * scaleUp
    body.lineCapStyle = .butt
    body.lineJoinStyle = .round
    body.stroke()
    // Pole tips.
    for x in [-170.0, 86.0] as [CGFloat] {
        let tip = NSBezierPath()
        tip.move(to: magnet(x, -161))
        tip.line(to: magnet(x + 84, -161))
        tip.line(to: magnet(x + 84, -109))
        tip.line(to: magnet(x, -109))
        tip.close()
        tip.fill()
    }
    // Attraction sparks.
    let sparks = NSBezierPath()
    for (a, b) in [((247.0, 121.0), (250.0, 152.0)), ((300.0, 142.0), (286.0, 166.0)), ((333.0, 181.0), (307.0, 194.0)),
                   ((461.0, 278.0), (464.0, 309.0)), ((514.0, 299.0), (500.0, 323.0)), ((547.0, 338.0), (521.0, 351.0))] {
        sparks.move(to: trace(CGFloat(a.0), CGFloat(a.1)))
        sparks.line(to: trace(CGFloat(b.0), CGFloat(b.1)))
    }
    sparks.lineWidth = 17 * scaleUp
    sparks.lineCapStyle = .round
    sparks.stroke()
    NSGraphicsContext.restoreGraphicsState()

    let data = rep.representation(using: .png, properties: [:])!
    if size == 512 { try data.write(to: URL(fileURLWithPath: "native/assets/logo-preview.png")) }
    let names: [String]
    if size == 16 { names = ["icon_16x16.png"] }
    else if size == 32 { names = ["icon_16x16@2x.png", "icon_32x32.png"] }
    else if size == 64 { names = ["icon_32x32@2x.png"] }
    else if size == 128 { names = ["icon_128x128.png"] }
    else if size == 256 { names = ["icon_128x128@2x.png", "icon_256x256.png"] }
    else if size == 512 { names = ["icon_256x256@2x.png", "icon_512x512.png"] }
    else { names = ["icon_512x512@2x.png"] }
    for name in names { try data.write(to: URL(fileURLWithPath: folder + "/" + name)) }
}
let task = Process()
task.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
task.arguments = ["-c", "icns", folder, "-o", "native/assets/Torrent.icns"]
try task.run(); task.waitUntilExit()
if task.terminationStatus != 0 { exit(1) }
