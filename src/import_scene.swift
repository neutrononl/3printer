import Foundation
import SceneKit
let url = URL(fileURLWithPath: CommandLine.arguments[1])
do {
	let scene = try SCNScene(url: url, options: [.checkConsistency: true])
	var triangles = [[[Float]]]()
	scene.rootNode.enumerateChildNodes { node, _ in
		guard let geometry = node.geometry, let source = geometry.sources(for: .vertex).first else { return }
		func vertex(_ index: Int) -> [Float] {
			let offset = source.dataOffset + index * source.dataStride
			let xyz: [Float] = (0..<3).map { axis in
				let start = offset + axis * source.bytesPerComponent
				return source.data.withUnsafeBytes { raw in
					if source.bytesPerComponent == 8 { return Float(raw.loadUnaligned(fromByteOffset: start, as: Double.self)) }
					return raw.loadUnaligned(fromByteOffset: start, as: Float.self)
				}
			}
			let p = node.convertPosition(SCNVector3(xyz[0], xyz[1], xyz[2]), to: nil)
			return [Float(p.x), Float(p.y), Float(p.z)]
		}
		for element in geometry.elements where element.primitiveType == .triangles {
			let count = element.primitiveCount * 3
			let indices: [Int] = (0..<count).map { i in element.data.withUnsafeBytes { raw in
				let offset = i * element.bytesPerIndex
				switch element.bytesPerIndex {
				case 1: return Int(raw.loadUnaligned(fromByteOffset: offset, as: UInt8.self))
				case 2: return Int(raw.loadUnaligned(fromByteOffset: offset, as: UInt16.self))
				default: return Int(raw.loadUnaligned(fromByteOffset: offset, as: UInt32.self))
				}
			} }
			for i in stride(from: 0, to: count, by: 3) { triangles.append([vertex(indices[i]), vertex(indices[i+1]), vertex(indices[i+2])]) }
		}
	}
	let data = try JSONSerialization.data(withJSONObject: triangles)
	FileHandle.standardOutput.write(data)
} catch {
	FileHandle.standardError.write(Data("Import failed: \(error.localizedDescription)\n".utf8))
	exit(1)
}
