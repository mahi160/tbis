import copy
import io
from pathlib import Path
import shutil
import stat
import tarfile
import tempfile
import unittest
import zipfile

import windows_sources as sources


MANIFEST = Path(__file__).with_name('windows-deps.json')


def make_tar(path, entries):
    with tarfile.open(path, 'w:gz') as archive:
        for name, value in entries:
            member = tarfile.TarInfo(name)
            if isinstance(value, tuple):
                member.type, member.linkname = value
                archive.addfile(member)
            else:
                member.size = len(value)
                archive.addfile(member, io.BytesIO(value))


class ManifestTests(unittest.TestCase):
    def setUp(self):
        self.manifest = sources.read_json(MANIFEST)

    def test_closed_pinned_manifest(self):
        self.assertEqual(sources.validate_manifest(self.manifest), self.manifest)

    def test_missing_and_unexpected_sources(self):
        for change in ('missing', 'extra', 'duplicate'):
            with self.subTest(change=change):
                manifest = copy.deepcopy(self.manifest)
                if change == 'missing':
                    manifest['sources'].pop()
                else:
                    component = copy.deepcopy(manifest['sources'][0])
                    if change == 'extra':
                        component['name'] = 'unowned'
                    manifest['sources'].append(component)
                with self.assertRaises(ValueError):
                    sources.validate_manifest(manifest)

    def test_unpinned_or_insecure_downloads(self):
        for key, value in (('sha256', ''), ('sha256', 'x' * 64),
                           ('url', 'http://example.org/source.tar.gz'),
                           ('archive', '../source.tar.gz'), ('version', '')):
            with self.subTest(key=key, value=value):
                manifest = copy.deepcopy(self.manifest)
                manifest['sources'][0][key] = value
                with self.assertRaises(ValueError):
                    sources.validate_manifest(manifest)

    def test_build_tool_pins_required(self):
        self.manifest['tools'].pop()
        with self.assertRaises(ValueError):
            sources.validate_manifest(self.manifest)


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.archive = self.root / 'source.tar.gz'
        self.output = self.root / 'output'

    def extract(self):
        sources.extract_archive(self.archive, self.output, sources.digest(self.archive))

    def test_regular_files_and_internal_links_materialized(self):
        make_tar(self.archive, [
            ('project/src/code.c', b'code'),
            ('project/link.c', (tarfile.SYMTYPE, 'src/code.c')),
            ('project/hard.c', (tarfile.LNKTYPE, 'project/src/code.c')),
        ])
        self.extract()
        for name in ('src/code.c', 'link.c', 'hard.c'):
            self.assertEqual((self.output / name).read_bytes(), b'code')
            self.assertFalse((self.output / name).is_symlink())

    def test_checksum_before_extraction(self):
        make_tar(self.archive, [('project/code.c', b'code')])
        with self.assertRaisesRegex(ValueError, 'Checksum'):
            sources.extract_archive(self.archive, self.output, '0' * 64)
        self.assertFalse(self.output.exists())

    def test_unsafe_paths_rejected_before_any_writes(self):
        for name in ('../escape', '/absolute', 'project/../escape',
                     'C:/escape', 'project/back\\slash', 'project/file:stream',
                     'project/CON.txt', 'project/trailing.', 'project/trailing ',
                     'project//empty'):
            with self.subTest(name=name):
                make_tar(self.archive, [('project/good', b'ok'), (name, b'bad')])
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertFalse(self.output.exists())

    def test_dangerous_links_and_special_files_rejected(self):
        for value in ((tarfile.SYMTYPE, '../../escape'),
                      (tarfile.SYMTYPE, '/absolute'),
                      (tarfile.SYMTYPE, 'C:/escape'),
                      (tarfile.LNKTYPE, '../escape'),
                      (tarfile.SYMTYPE, 'link'),
                      (tarfile.FIFOTYPE, '')):
            with self.subTest(value=value):
                make_tar(self.archive, [('project/link', value)])
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertFalse(self.output.exists())

    def test_case_collisions_and_link_parent_rejected(self):
        examples = (
            [('project/file', b'a'), ('project/FILE', b'b')],
            [('project/Src/a', b'a'), ('project/src/b', b'b')],
            [('project/link', (tarfile.SYMTYPE, 'file')),
             ('project/file', b'a'), ('project/link/child', b'b')],
        )
        for entries in examples:
            with self.subTest(entries=entries):
                make_tar(self.archive, entries)
                with self.assertRaises(ValueError):
                    self.extract()
                self.assertFalse(self.output.exists())

    def test_existing_destination_not_overwritten(self):
        make_tar(self.archive, [('project/code.c', b'code')])
        self.output.mkdir()
        (self.output / 'owned').write_bytes(b'keep')
        with self.assertRaises(ValueError):
            self.extract()
        self.assertEqual((self.output / 'owned').read_bytes(), b'keep')

    def test_zip_symlinks_rejected(self):
        archive = self.root / 'tool.zip'
        with zipfile.ZipFile(archive, 'w') as output:
            member = zipfile.ZipInfo('tool/link')
            member.external_attr = (stat.S_IFLNK | 0o777) << 16
            output.writestr(member, '../escape')
        with self.assertRaises(ValueError):
            sources.extract_archive(archive, self.output, sources.digest(archive))
        self.assertFalse(self.output.exists())

    def test_nested_license_and_freetype_notices(self):
        names = ('docs/FTL.TXT', 'docs/GPLv2.TXT', 'nested/LICENSES/MIT.txt',
                 'nested/NOTICE', 'nested/COPYRIGHT', 'code.c')
        for name in names:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'notice')
        self.assertEqual({path.relative_to(self.root).as_posix() for path in sources.notices(self.root)}, set(names[:-1]))


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / 'repo'
        self.deps = self.root / 'deps'
        self.source = self.deps / 'corresponding-source'
        self.binary = self.deps / 'mpv/libmpv-2.dll'
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(b'actual fixture DLL')
        (self.binary.parent / 'mpv.lib').write_bytes(b'import library')
        manifest = sources.read_json(MANIFEST)
        for item in manifest['sources']:
            archive = self.source / 'sources' / item['archive']
            archive.parent.mkdir(parents=True, exist_ok=True)
            make_tar(archive, [('project/code.c', item['name'].encode())])
            item['sha256'] = sources.digest(archive)
            license_path = self.source / 'licenses' / item['name'] / 'LICENSE'
            license_path.parent.mkdir(parents=True)
            license_path.write_bytes(b'license')
        for name in sources.RECIPES:
            path = self.repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'recipe input')
        sources.write_json(self.repo / 'scripts/windows-deps.json', manifest)
        for name in sources.RECIPES:
            destination = self.source / 'recipes' / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(self.repo / name, destination)
        sources.bind_inventory(self.repo, self.deps)

    def test_actual_binary_source_and_recipe_binding(self):
        self.assertEqual(sources.verify_inventory(self.repo, self.deps), self.source)
        inventory = sources.read_json(self.source / 'inventory.json')
        self.assertEqual(inventory['binarySha256'], sources.digest(self.binary))
        self.assertEqual({component['name'] for component in inventory['components']}, sources.COMPONENTS)

    def test_prepared_source_change_rejected_with_unchanged_binaries(self):
        path = self.deps / 'source/mpv/code.c'
        path.parent.mkdir(parents=True)
        path.write_bytes(b'prepared source')
        sources.write_json(self.source / 'recipes/prepared-source-sha256.json',
                           sources.file_records(self.deps / 'source'))
        binaries = sources.file_records(self.deps / 'mpv')
        sources.validate_prepared_source(self.deps)
        path.write_bytes(b'changed source')
        with self.assertRaisesRegex(ValueError, 'Prepared source tree changed'):
            sources.validate_prepared_source(self.deps)
        self.assertEqual(sources.file_records(self.deps / 'mpv'), binaries)

    def test_replaced_binary_rejected(self):
        self.binary.write_bytes(b'other build')
        with self.assertRaises(ValueError):
            sources.verify_inventory(self.repo, self.deps)

    def test_current_recipe_change_rejected(self):
        (self.repo / 'scripts/build-windows-mpv.ps1').write_bytes(b'changed recipe')
        with self.assertRaises(ValueError):
            sources.verify_inventory(self.repo, self.deps)

    def test_manifest_change_rejected(self):
        manifest = sources.read_json(self.repo / 'scripts/windows-deps.json')
        manifest['buildOptions'].append('changed=true')
        sources.write_json(self.repo / 'scripts/windows-deps.json', manifest)
        with self.assertRaises(ValueError):
            sources.verify_inventory(self.repo, self.deps)

    def test_source_or_license_tampering_rejected(self):
        for relative in ('sources/mpv.tar.gz', 'licenses/mpv/LICENSE'):
            with self.subTest(relative=relative):
                path = self.source / relative
                original = path.read_bytes()
                path.write_bytes(b'tampered')
                with self.assertRaises(ValueError):
                    sources.verify_inventory(self.repo, self.deps)
                path.write_bytes(original)

    def test_missing_component_or_empty_license_inventory_rejected(self):
        path = self.source / 'inventory.json'
        original = sources.read_json(path)
        for change in ('component', 'licenses'):
            with self.subTest(change=change):
                value = copy.deepcopy(original)
                if change == 'component':
                    value['components'].pop()
                else:
                    value['components'][0]['licenses'] = []
                sources.write_json(path, value)
                with self.assertRaises(ValueError):
                    sources.verify_inventory(self.repo, self.deps)
        sources.write_json(path, original)

    def test_added_undisclosed_source_file_rejected(self):
        (self.source / 'sources/undisclosed.tar').write_bytes(b'unowned')
        with self.assertRaises(ValueError):
            sources.verify_inventory(self.repo, self.deps)


if __name__ == '__main__':
    unittest.main()
