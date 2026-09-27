// Renders assets/icon.svg to assets/icon.png (1024x1024, transparent corners),
// the source scripts/bundle.sh builds the .icns from. Run after editing the SVG:
//   swift scripts/render-icon.swift
import AppKit

let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let svg = root.appendingPathComponent("assets/icon.svg")
let png = root.appendingPathComponent("assets/icon.png")
let size = 1024

guard let image = NSImage(contentsOf: svg) else {
    fatalError("can't load \(svg.path)")
}
guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
) else {
    fatalError("can't allocate bitmap")
}
bitmap.size = NSSize(width: size, height: size)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
// bitmap starts fully transparent; only the icon's own shapes are drawn
image.draw(in: NSRect(x: 0, y: 0, width: size, height: size))
NSGraphicsContext.restoreGraphicsState()

guard let data = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("can't encode PNG")
}
try data.write(to: png)
print("wrote \(png.path)")
