import os
import tempfile
import unittest
import zipfile
from import_3mf import load


class ImportTests(unittest.TestCase):
	mesh = '<object id="1"><mesh><vertices><vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/></vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object>'

	def read(self, resources, build):
		with tempfile.TemporaryDirectory() as directory:
			path = os.path.join(directory, 'model.3mf')
			with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
				archive.writestr('_rels/.rels', '<Relationships><Relationship Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/3dmodel.model"/></Relationships>')
				archive.writestr('3D/3dmodel.model', '<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" unit="inch"><resources>' + resources + '</resources><build>' + build + '</build></model>')
			return load(path)

	def test_components_units_and_instances(self):
		resources = self.mesh + '<object id="2"><components><component objectid="1" transform="1 0 0 0 1 0 0 0 1 2 0 0"/></components></object>'
		triangles = self.read(resources, '<item objectid="2" transform="1 0 0 0 1 0 0 0 1 0 3 0"/><item objectid="1"/>')
		self.assertEqual(len(triangles), 2)
		self.assertAlmostEqual(triangles[0][0][0], 50.8)
		self.assertAlmostEqual(triangles[0][0][1], 76.2)
		self.assertEqual(triangles[1][1], [25.4, 0, 0])

	def test_invalid_index(self):
		with self.assertRaises(ValueError):
			self.read(self.mesh.replace('v3="2"', 'v3="99"'), '<item objectid="1"/>')

	def test_cycle(self):
		with self.assertRaises(ValueError):
			self.read('<object id="1"><components><component objectid="1"/></components></object>', '<item objectid="1"/>')


if __name__ == '__main__':
	unittest.main()
