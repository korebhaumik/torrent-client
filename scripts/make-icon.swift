import AppKit
import Foundation

let folder = "native/assets/Torrent.iconset"
try FileManager.default.createDirectory(atPath: folder, withIntermediateDirectories: true)
for size in [16, 32, 64, 128, 256, 512, 1024] {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let scale = CGFloat(size) / 1024
    let transform = NSAffineTransform()
    transform.scale(by: scale)
    transform.concat()
    NSColor(calibratedRed: 76/255, green: 115/255, blue: 236/255, alpha: 1).setFill()
    NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 200, yRadius: 200).fill()
    let path = NSBezierPath()
    path.move(to: NSPoint(x: 280, y: 390))
    path.line(to: NSPoint(x: 535, y: 645))
    path.curve(to: NSPoint(x: 715, y: 465), controlPoint1: NSPoint(x: 710, y: 820), controlPoint2: NSPoint(x: 890, y: 640))
    path.line(to: NSPoint(x: 460, y: 210))
    path.lineWidth = 64
    path.lineCapStyle = .round
    NSColor.white.setStroke()
    path.stroke()
    let bars = NSBezierPath()
    bars.move(to: NSPoint(x: 400, y: 510)); bars.line(to: NSPoint(x: 493, y: 417))
    bars.move(to: NSPoint(x: 580, y: 330)); bars.line(to: NSPoint(x: 673, y: 423))
    bars.lineWidth = 24; bars.stroke()
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
