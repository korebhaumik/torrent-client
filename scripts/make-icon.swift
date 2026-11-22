import AppKit
import Foundation

// Draws the same artwork as native/assets/brand.svg and `brand_mark` in
// native/src/design.rs. Coordinates use the SVG's 1024-unit, y-down space.
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
    let scale = CGFloat(size) / 1024
    let transform = NSAffineTransform()
    transform.scale(by: scale)
    transform.translateX(by: 0, yBy: 1024)
    transform.scaleX(by: 1, yBy: -1)
    transform.concat()

    let tile = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 200, yRadius: 200)
    NSGraphicsContext.saveGraphicsState()
    tile.addClip()
    NSGradient(starting: color(124, 92, 255), ending: color(44, 110, 245))!
        .draw(from: NSPoint(x: 160, y: 96), to: NSPoint(x: 864, y: 928),
              options: [.drawsBeforeStartingLocation, .drawsAfterEndingLocation])
    NSGradient(starting: color(255, 255, 255, 0.16), ending: color(255, 255, 255, 0))!
        .draw(from: NSPoint(x: 512, y: 64), to: NSPoint(x: 512, y: 560), options: [.drawsBeforeStartingLocation])
    NSGraphicsContext.restoreGraphicsState()

    NSColor.white.setStroke()
    func stroke(_ points: [[(CGFloat, CGFloat)]], _ width: CGFloat) {
        let path = NSBezierPath()
        for line in points {
            path.move(to: NSPoint(x: line[0].0, y: line[0].1))
            for p in line.dropFirst() { path.line(to: NSPoint(x: p.0, y: p.1)) }
        }
        path.lineWidth = width
        path.lineCapStyle = .round
        path.lineJoinStyle = .round
        path.stroke()
    }
    // A "T" whose stem becomes a download arrow, landing on a baseline.
    stroke([[(300, 270), (724, 270)], [(512, 270), (512, 660)], [(396, 556), (512, 672), (628, 556)]], 64)
    stroke([[(330, 790), (694, 790)]], 48)
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
