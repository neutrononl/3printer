"""Read 3MF geometry with Python's standard library; invoked by the desktop app."""
import json
import math
import posixpath
import sys
import zipfile
import xml.etree.ElementTree as ET

IDENTITY = [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]


def local(tag):
	return tag.rsplit('}', 1)[-1]


def transform(node):
	values = list(map(float, node.get('transform', '').split())) or IDENTITY
	if len(values) != 12 or not all(map(math.isfinite, values)):
		raise ValueError('Invalid 3MF transform')
	return values


def apply(v, m):
	return [sum(v[j] * m[j * 3 + i] for j in range(3)) + m[9 + i] for i in range(3)]


def load(path):
	with zipfile.ZipFile(path) as archive:
		def xml(name):
			info = archive.getinfo(name)
			if info.file_size > 128 * 1024 * 1024:
				raise ValueError('3MF model is too large')
			data = archive.read(info)
			if b'<!DOCTYPE' in data.upper() or b'<!ENTITY' in data.upper():
				raise ValueError('Unsupported XML declarations')
			return ET.fromstring(data)

		root_path = None
		for rel in xml('_rels/.rels'):
			if rel.get('Type', '').endswith('/3dmodel') and rel.get('TargetMode') != 'External':
				root_path = posixpath.normpath(rel.attrib['Target'].lstrip('/'))
				break
		if not root_path:
			raise ValueError('3MF has no model relationship')
		models = {}

		def model(name):
			if name not in models:
				root = xml(name)
				units = {'micron': .001, 'millimeter': 1, 'centimeter': 10, 'inch': 25.4, 'foot': 304.8, 'meter': 1000}
				scale = units[root.get('unit', 'millimeter')]
				objects = {n.attrib['id']: n for n in root.iter() if local(n.tag) == 'object'}
				models[name] = (root, scale, objects)
			return models[name]

		triangles = []

		def expand(name, object_id, chain, stack):
			key = (name, object_id)
			if key in stack or len(stack) >= 64:
				raise ValueError('Cyclic or excessively nested 3MF components')
			_, scale, objects = model(name)
			obj = objects[object_id]
			for child in obj:
				if local(child.tag) == 'mesh':
					vertices = [[float(n.attrib[a]) for a in ('x', 'y', 'z')] for n in child.iter() if local(n.tag) == 'vertex']
					for n in child.iter():
						if local(n.tag) != 'triangle':
							continue
						tri = []
						for a in ('v1', 'v2', 'v3'):
							i = int(n.attrib[a])
							if i < 0 or i >= len(vertices):
								raise ValueError('3MF vertex index out of range')
							v = vertices[i]
							for matrix in chain:
								v = apply(v, matrix)
							tri.append([x * scale for x in v])
						triangles.append(tri)
						if len(triangles) > 1_000_000:
							raise ValueError('Model exceeds one million triangles')
				elif local(child.tag) == 'components':
					for component in child:
						ref = next((v for k, v in component.attrib.items() if local(k) == 'path'), None)
						target = posixpath.normpath(ref.lstrip('/') if ref and ref.startswith('/') else posixpath.join(posixpath.dirname(name), ref)) if ref else name
						expand(target, component.attrib['objectid'], [transform(component)] + chain, stack + [key])

		root, _, _ = model(root_path)
		for build in root:
			if local(build.tag) == 'build':
				for item in build:
					if item.get('printable', '1') not in ('0', 'false'):
						expand(root_path, item.attrib['objectid'], [transform(item)], [])
		return triangles


if __name__ == '__main__':
	try:
		json.dump(load(sys.argv[1]), sys.stdout, allow_nan=False, separators=(',', ':'))
	except Exception as error:
		print('Cannot import 3MF: ' + str(error), file=sys.stderr)
		sys.exit(1)
