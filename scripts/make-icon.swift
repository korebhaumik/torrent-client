import AppKit
import Foundation

// Draws the same artwork as native/assets/brand.svg and `brand_mark` in
// native/src/design.rs. Coordinates use the SVG's 64-unit, y-down space.
let folder = "native/assets/Torrent.iconset"
try FileManager.default.createDirectory(atPath: folder, withIntermediateDirectories: true)

func color(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat, _ a: CGFloat = 1) -> NSColor {
    NSColor(calibratedRed: r / 255, green: g / 255, blue: b / 255, alpha: a)
}

for size in [16, 32, 64, 128, 256, 512, 1024] {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let scale = CGFloat(size) / 64
    let transform = NSAffineTransform()
    transform.scale(by: scale)
    transform.translateX(by: 0, yBy: 64)
    transform.scaleX(by: 1, yBy: -1)
    transform.concat()

    func polygon(_ points: [(CGFloat, CGFloat)]) -> NSBezierPath {
        let path = NSBezierPath()
        path.move(to: NSPoint(x: points[0].0, y: points[0].1))
        for p in points.dropFirst() { path.line(to: NSPoint(x: p.0, y: p.1)) }
        path.close()
        return path
    }
    // SVG objectBoundingBox gradients run between corners of each shape's bounds.
    func fill(_ path: NSBezierPath, _ gradient: NSGradient, from: NSPoint, to: NSPoint) {
        NSGraphicsContext.saveGraphicsState()
        path.addClip()
        gradient.draw(from: from, to: to, options: [.drawsBeforeStartingLocation, .drawsAfterEndingLocation])
        NSGraphicsContext.restoreGraphicsState()
    }

    // Outer drop shield.
    let drop = NSBezierPath()
    drop.move(to: NSPoint(x: 32, y: 10))
    drop.curve(to: NSPoint(x: 52, y: 34), controlPoint1: NSPoint(x: 45, y: 10), controlPoint2: NSPoint(x: 52, y: 20))
    drop.curve(to: NSPoint(x: 32, y: 54), controlPoint1: NSPoint(x: 52, y: 46), controlPoint2: NSPoint(x: 43, y: 54))
    drop.curve(to: NSPoint(x: 12, y: 34), controlPoint1: NSPoint(x: 21, y: 54), controlPoint2: NSPoint(x: 12, y: 46))
    drop.curve(to: NSPoint(x: 32, y: 10), controlPoint1: NSPoint(x: 12, y: 20), controlPoint2: NSPoint(x: 19, y: 10))
    drop.close()
    let main = NSGradient(colors: [color(56, 189, 248), color(99, 102, 241), color(139, 92, 246)],
                          atLocations: [0, 0.5, 1], colorSpace: .deviceRGB)!
    fill(drop, main, from: NSPoint(x: 12, y: 10), to: NSPoint(x: 52, y: 54))

    // Arrow cutout.
    color(14, 15, 20).setFill()
    polygon([(32, 20), (44, 33), (37, 33), (37, 44), (27, 44), (27, 33), (20, 33)]).fill()

    // Inner highlight arrow at 90% opacity.
    let accent = NSGradient(starting: color(99, 102, 241, 0.9), ending: color(56, 189, 248, 0.9))!
    fill(polygon([(32, 25), (39, 33), (34, 33), (34, 40), (30, 40), (30, 33), (25, 33)]), accent,
         from: NSPoint(x: 25, y: 40), to: NSPoint(x: 39, y: 25))
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
