import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import urllib.request
import zipfile


COMPONENTS = {
    'mpv', 'ffmpeg', 'libplacebo', 'libass', 'dav1d', 'freetype2',
    'fribidi', 'harfbuzz', 'zlib', 'wrapdb', 'glad', 'jinja',
    'markupsafe', 'fast_float', 'vulkan_headers',
}
RECIPES = (
    '.gitattributes',
    'scripts/windows-deps.json', 'scripts/windows-common.ps1',
    'scripts/setup-windows-deps.ps1', 'scripts/build-windows-mpv.ps1',
    'scripts/bundle-windows.ps1', 'scripts/windows_sources.py',
    'scripts/windows_sources_tests.py', '.github/workflows/windows.yml',
    '.github/workflows/mirror-windows-deps.yml',
    'scripts/patches/mpv-d3d11-hwdec-helpers.patch',
)
SYSTEM_LIBRARIES = {
    'advapi32', 'avrt', 'bcrypt', 'crypt32', 'd3d11', 'd3d12', 'd3d9',
    'd3dcompiler', 'dbghelp', 'dwmapi', 'dwrite', 'dxgi', 'dxva2', 'gdi32',
    'imm32', 'kernel32', 'mfplat', 'mfuuid', 'ncrypt', 'ntdll', 'ole32',
    'oleaut32', 'opengl32', 'pathcch', 'psapi', 'secur32', 'setupapi',
    'shcore', 'shell32', 'shlwapi', 'strmiids', 'user32', 'userenv', 'uuid',
    'uxtheme', 'version', 'winmm', 'winspool', 'comdlg32', 'ws2_32', 'libcmt', 'libcpmt',
    'libucrt', 'libvcruntime', 'oldnames',
}


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def safe_name(name):
    parts = name.rstrip('/').split('/')
    if not parts or any(part in ('', '.', '..') for part in parts):
        raise ValueError(f'Unsafe archive path: {name}')
    for part in parts:
        device = part.partition('.')[0].rstrip(' ').upper()
        if (re.search(r'[\x00-\x1f<>:"\\|?*]', part) or part.endswith((' ', '.'))
                or device in {'CON', 'PRN', 'AUX', 'NUL', 'CONIN$', 'CONOUT$'}
                or re.fullmatch(r'(?:COM|LPT)[1-9¹²³]', device)):
            raise ValueError(f'Windows-invalid archive path: {name}')
    return PurePosixPath(*parts)


def validate_manifest(manifest):
    if manifest['schemaVersion'] != 1:
        raise ValueError('Unsupported source manifest schema')
    if {item['name'] for item in manifest['sources']} != COMPONENTS:
        raise ValueError('Source manifest does not describe the closed component set')
    if {item['name'] for item in manifest['tools']} != {'meson', 'ninja', 'nasm'}:
        raise ValueError('Missing pinned build tools')
    names = set()
    archives = set()
    for item in manifest['sources'] + manifest['tools']:
        if item['name'] in names or item['archive'].casefold() in archives:
            raise ValueError('Duplicate source/tool identity')
        names.add(item['name'])
        archives.add(item['archive'].casefold())
        if (not re.fullmatch(r'[a-z][a-z0-9_]*', item['name']) or not item['version']
                or not re.fullmatch(r'[0-9a-f]{64}', item['sha256'])
                or not item['url'].startswith('https://')
                or len(safe_name(item['archive']).parts) != 1):
            raise ValueError(f'Incomplete source pin: {item["name"]}')
    if len(manifest['buildOptions']) != len(set(manifest['buildOptions'])):
        raise ValueError('Duplicate build options')
    return manifest


def download(item, destination):
    destination = Path(destination)
    if not destination.exists():
        request = urllib.request.Request(item['url'], headers={'User-Agent': 'tbis-source-builder'})
        partial = destination.with_name(destination.name + '.partial')
        try:
            with urllib.request.urlopen(request, timeout=120) as response, partial.open('wb') as output:
                if not response.url.startswith('https://'):
                    raise ValueError('Source download redirected away from HTTPS')
                shutil.copyfileobj(response, output)
            if digest(partial) != item['sha256']:
                raise ValueError(f'Checksum mismatch: {item["name"]}')
            partial.replace(destination)
        finally:
            partial.unlink(missing_ok=True)
    if digest(destination) != item['sha256']:
        raise ValueError(f'Checksum mismatch: {item["name"]}')


def extract_archive(archive, destination, sha256, strip_root=True):
    archive, destination = Path(archive), Path(destination)
    if digest(archive) != sha256:
        raise ValueError(f'Checksum mismatch: {archive.name}')
    if destination.exists():
        raise ValueError(f'Extraction destination exists: {destination}')
    entries = {}
    folded = set()
    prefixes = {}
    is_zip = zipfile.is_zipfile(archive)
    with (zipfile.ZipFile(archive) if is_zip else tarfile.open(archive)) as source:
        members = source.infolist() if is_zip else source.getmembers()
        for member in members:
            name = member.filename if is_zip else member.name
            path = safe_name(name)
            if str(path).casefold() in folded:
                raise ValueError(f'Duplicate archive path: {name}')
            folded.add(str(path).casefold())
            for prefix in (path, *path.parents[:-1]):
                key = str(prefix).casefold()
                if key in prefixes and prefixes[key] != prefix:
                    raise ValueError(f'Case-colliding archive path: {name}')
                prefixes[key] = prefix
            if is_zip:
                mode = member.external_attr >> 16
                kind = 'dir' if member.is_dir() else 'file'
                if stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR):
                    raise ValueError(f'Unsafe zip member type: {name}')
            else:
                kind = ('dir' if member.isdir() else 'file' if member.isfile()
                        else 'link' if member.issym() or member.islnk() else None)
                if kind is None:
                    raise ValueError(f'Unsafe archive member type: {name}')
            entries[path] = (member, kind)
        roots = {path.parts[0] for path in entries}
        if strip_root and (len(roots) != 1 or any(len(p.parts) == 1 and k != 'dir' for p, (_, k) in entries.items())):
            raise ValueError('Expected one archive root directory')
        for path in entries:
            for parent in path.parents:
                if parent in entries and entries[parent][1] != 'dir':
                    raise ValueError(f'Archive member below a file or link: {path}')

        def resolve(path, seen):
            if path in seen or path not in entries:
                raise ValueError(f'Broken or cyclic archive link: {path}')
            member, kind = entries[path]
            if kind != 'link':
                if kind != 'file':
                    raise ValueError(f'Archive link does not target a file: {path}')
                return member
            target = member.linkname
            if '\\' in target or PurePosixPath(target).is_absolute() or PureWindowsPath(target).drive:
                raise ValueError(f'Unsafe archive link: {path}')
            parts = list(path.parent.parts) if member.issym() else []
            for part in target.split('/'):
                if part == '..':
                    if not parts:
                        raise ValueError(f'Escaping archive link: {path}')
                    parts.pop()
                elif part not in ('', '.'):
                    parts.append(part)
            resolved = safe_name('/'.join(parts))
            if strip_root and resolved.parts[0] != path.parts[0]:
                raise ValueError(f'Escaping source root link: {path}')
            return resolve(resolved, seen | {path})

        resolved = {path: resolve(path, set()) for path, (_, kind) in entries.items() if kind == 'link'}
        destination.mkdir(parents=True)
        for path, (member, kind) in entries.items():
            relative = Path(*path.parts[1:]) if strip_root else Path(*path.parts)
            output = destination / relative
            if kind == 'dir':
                output.mkdir(parents=True, exist_ok=True)
            else:
                output.parent.mkdir(parents=True, exist_ok=True)
                with (source.open(member) if is_zip else source.extractfile(resolved.get(path, member))) as stream, output.open('wb') as target:
                    shutil.copyfileobj(stream, target)
                if not is_zip:
                    output.chmod(member.mode & 0o777)


def file_records(root):
    root = Path(root)
    result = []
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'Unexpected source symlink: {path}')
        if path.is_file():
            result.append({'path': path.relative_to(root).as_posix(), 'sha256': digest(path)})
    return result


def notices(root):
    for path in sorted(Path(root).rglob('*')):
        if path.is_file() and (
            re.match(r'(?i)^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|FTL\.|GPL|LGPL|OFL)', path.name)
            or any(re.fullmatch(r'(?i)licen[cs]es?', part) for part in path.relative_to(root).parts[:-1])
        ):
            yield path


def prepare(repo, deps):
    repo, deps = Path(repo), Path(deps)
    manifest = validate_manifest(read_json(repo / 'scripts/windows-deps.json'))
    source = deps / 'corresponding-source'
    source.mkdir(parents=True)
    archives = source / 'sources'
    archives.mkdir()
    unpacked = deps / 'unpacked'
    for item in manifest['sources']:
        archive = archives / item['archive']
        download(item, archive)
        extract_archive(archive, unpacked / item['name'], item['sha256'])
        license_root = source / 'licenses' / item['name']
        found = list(notices(unpacked / item['name']))
        if not found:
            raise ValueError(f'No license notices: {item["name"]}')
        for path in found:
            output = license_root / path.relative_to(unpacked / item['name'])
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, output)
    tools = deps / 'tools'
    tools.mkdir()
    for item in manifest['tools']:
        archive = tools / item['archive']
        download(item, archive)
        extract_archive(archive, tools / item['name'], item['sha256'], strip_root=False)
    shutil.copytree(unpacked / 'mpv', deps / 'source/mpv')
    subprocess.run(
        ['git', 'apply', str((repo / 'scripts/patches/mpv-d3d11-hwdec-helpers.patch').resolve())],
        cwd=deps / 'source/mpv',
        env={**os.environ, 'GIT_CEILING_DIRECTORIES': str((deps / 'source').resolve())},
        check=True,
    )
    subprojects = deps / 'source/mpv/subprojects'
    subprojects.mkdir(exist_ok=True)
    for item in manifest['sources']:
        if 'provides' not in item:
            continue
        name = item['name']
        shutil.copytree(unpacked / name, subprojects / name)
        provide = ('dav1d = dav1d_dep' if name == 'dav1d'
                   else 'dependency_names = ' + ', '.join(item['provides']))
        (subprojects / f'{name}.wrap').write_text(
            f'[wrap-file]\ndirectory = {name}\n\n[provide]\n{provide}\n', encoding='utf-8')
    zlib_overlay = unpacked / 'wrapdb/subprojects/packagefiles/zlib'
    shutil.copytree(zlib_overlay, subprojects / 'zlib', dirs_exist_ok=True)
    for name, directory in (
        ('glad', 'glad'), ('jinja', 'jinja'), ('markupsafe', 'markupsafe'),
        ('fast_float', 'fast_float'), ('vulkan_headers', 'Vulkan-Headers'),
    ):
        destination = subprojects / 'libplacebo/3rdparty' / directory
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(unpacked / name, destination)
    for path in subprojects.rglob('*.wrap'):
        if path.parent != subprojects:
            path.unlink()
    for path in subprojects.rglob('wrapdb.json'):
        path.unlink()
    recipe_root = source / 'recipes'
    for name in RECIPES:
        output = recipe_root / name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo / name, output)
    write_json(recipe_root / 'prepared-source-sha256.json', file_records(deps / 'source'))
    generated = recipe_root / 'generated'
    generated.mkdir()
    for path in subprojects.glob('*.wrap'):
        shutil.copy2(path, generated / path.name)
    shutil.copytree(zlib_overlay, generated / 'zlib')


def validate_prepared_source(deps):
    deps = Path(deps)
    records = read_json(deps / 'corresponding-source/recipes/prepared-source-sha256.json')
    # Meson creates this coordination file during configuration; it is not source.
    current = [record for record in file_records(deps / 'source')
               if record['path'] != 'mpv/subprojects/.wraplock']
    if current != records:
        raise ValueError('Prepared source tree changed')


def audit_graph(deps):
    deps = Path(deps)
    validate_prepared_source(deps)
    info = deps / 'build/meson-info'
    expected = {'ffmpeg', 'libplacebo', 'libass', 'dav1d', 'freetype2', 'fribidi', 'harfbuzz', 'zlib'}
    projects = {item['name'] for item in read_json(info / 'intro-projectinfo.json')['subprojects']}
    if projects != expected:
        raise ValueError(f'Unexpected source dependency graph: {projects ^ expected}')
    targets = read_json(info / 'intro-targets.json')
    shared = [target for target in targets if target['type'] in ('shared library', 'shared module')]
    if len(shared) != 1 or shared[0]['subproject'] or shared[0]['name'] != 'mpv':
        raise ValueError('Only the top-level libmpv may be shared')
    for dep in read_json(info / 'intro-dependencies.json'):
        if dep['type'] == 'internal':
            continue
        if dep['type'] == 'system' and (
            dep['name'] == 'threads' or (
                dep['name'] == 'gl' and dep['link_args'] == ['-lopengl32']
                and not dep['compile_args']
            )
        ):
            continue
        raise ValueError(f'Undisclosed external dependency: {dep["name"]} ({dep["type"]})')
    options = {option['name']: option['value'] for option in read_json(info / 'intro-buildoptions.json')}
    if options['default_library'] != 'shared':
        raise ValueError('Unexpected library linkage: mpv')
    if options['b_vscrt'] != 'mt':
        raise ValueError('Non-static MSVC runtime: mpv')
    compilers = read_json(info / 'intro-compilers.json')['host']
    for language in ('c', 'cpp'):
        if compilers[language]['id'] != 'clang' or compilers[language]['linker_id'] != 'lld-link':
            raise ValueError('The source recipe requires GNU-frontend Clang with the MSVC linker target')
    machine = read_json(info / 'intro-machines.json')['host']
    if machine['system'] != 'windows' or machine['cpu_family'] != 'x86_64':
        raise ValueError('The source recipe requires native Windows x64')
    built_libraries = {Path(filename).resolve() for target in targets
                       if target['type'] == 'static library' for filename in target['filename']}
    sdk_roots = [Path(os.environ[name]).resolve() for name in
                 ('VCToolsInstallDir', 'WindowsSdkDir', 'UniversalCRTSdkDir') if os.environ.get(name)]
    for target in targets:
        for group in target.get('target_sources', []):
            if (target['type'] in ('static library', 'shared library')
                    and group.get('machine') == 'host' and group.get('language') in ('c', 'cpp')):
                runtime = [arg for arg in group['parameters'] if arg.startswith('-fms-runtime-lib=')]
                if runtime != ['-fms-runtime-lib=static']:
                    raise ValueError(f'Non-static MSVC runtime: {target["name"]}')
            for arg in group.get('parameters', []):
                if arg.lower().startswith(('/out:', '/implib:', '-wl,/out:', '-wl,/implib:')):
                    continue
                if arg.lower().endswith(('.lib', '.a')):
                    path = Path(arg)
                    resolved = (path if path.is_absolute() else deps / 'build' / path).resolve()
                    if resolved in built_libraries:
                        continue
                    if path.stem.lower() in SYSTEM_LIBRARIES and (
                        len(path.parts) == 1 or any(resolved.is_relative_to(root) for root in sdk_roots)
                    ):
                        continue
                    raise ValueError(f'Undisclosed link library: {arg}')
                if arg.startswith('-l') and not arg.startswith('-link') and arg[2:] not in SYSTEM_LIBRARIES:
                    raise ValueError(f'Undisclosed link library: {arg}')
    config_root = deps / 'build/subprojects/ffmpeg'
    config = '\n'.join((config_root / name).read_text(encoding='utf-8') for name in ('config.h', 'config_components.h'))
    for feature in (
        'NETWORK', 'SCHANNEL', 'ZLIB', 'LIBDAV1D', 'D3D11VA', 'H264_DECODER',
        'HEVC_DECODER', 'H264_PARSER', 'HEVC_PARSER', 'AAC_DECODER', 'AAC_PARSER',
        'AC3_DECODER', 'EAC3_DECODER', 'AC3_PARSER', 'MP3_DECODER', 'FLAC_DECODER',
        'OPUS_DECODER', 'VORBIS_DECODER', 'ASS_DECODER', 'SUBRIP_DECODER',
        'LIBDAV1D_DECODER', 'PNG_ENCODER', 'MATROSKA_DEMUXER', 'MOV_DEMUXER',
        'FILE_PROTOCOL', 'TCP_PROTOCOL', 'HTTP_PROTOCOL', 'HTTPS_PROTOCOL', 'TLS_PROTOCOL',
    ):
        if not re.search(rf'^#define CONFIG_{feature} 1\s*$', config, re.MULTILINE):
            raise ValueError(f'Required FFmpeg feature absent: {feature}')
    return shared[0]['filename'][0]


def inventory(repo, deps):
    repo, deps = Path(repo), Path(deps)
    audit_graph(deps)
    source = deps / 'corresponding-source'
    build_records = source / 'recipes/build'
    build_records.mkdir()
    for name in ('meson-info', 'meson-logs'):
        shutil.copytree(deps / 'build' / name, build_records / name)
    for path in (deps / 'build').rglob('*'):
        if path.is_file() and path.name in ('config.h', 'config_components.h', 'config.asm', 'config.log', 'build.ninja', 'compile_commands.json'):
            output = build_records / path.relative_to(deps / 'build')
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, output)
    for name in ('native.ini', 'toolchain.json', 'build-command.json', 'build.log'):
        shutil.copy2(deps / name, build_records / name)
    bind_inventory(repo, deps)


def bind_inventory(repo, deps):
    repo, deps = Path(repo), Path(deps)
    source = deps / 'corresponding-source'
    manifest = validate_manifest(read_json(repo / 'scripts/windows-deps.json'))
    records = file_records(source)
    recipes = [record for record in records if record['path'].startswith('recipes/')]
    components = []
    for item in manifest['sources']:
        components.append({
            'name': item['name'], 'version': item['version'], 'url': item['url'],
            'sources': [{'path': 'sources/' + item['archive'], 'sha256': item['sha256']}],
            'recipes': recipes,
            'licenses': [record for record in records if record['path'].startswith('licenses/' + item['name'] + '/')],
        })
    write_json(source / 'inventory.json', {
        'schemaVersion': 1,
        'binarySha256': digest(deps / 'mpv/libmpv-2.dll'),
        'binaries': file_records(deps / 'mpv'),
        'manifestSha256': digest(repo / 'scripts/windows-deps.json'),
        'recipeInputs': [{'path': name, 'sha256': digest(repo / name)} for name in RECIPES],
        'components': components, 'files': records,
    })
    verify_inventory(repo, deps)


def verify_inventory(repo, deps):
    repo, deps = Path(repo), Path(deps)
    source = deps / 'corresponding-source'
    manifest = validate_manifest(read_json(repo / 'scripts/windows-deps.json'))
    value = read_json(source / 'inventory.json')
    current_inputs = [{'path': name, 'sha256': digest(repo / name)} for name in RECIPES]
    if (value['schemaVersion'] != 1
            or value['binarySha256'] != digest(deps / 'mpv/libmpv-2.dll')
            or value['binaries'] != file_records(deps / 'mpv')
            or value['manifestSha256'] != digest(repo / 'scripts/windows-deps.json')
            or value['recipeInputs'] != current_inputs):
        raise ValueError('Binary, manifest or current recipe does not match Corresponding Source')
    records = [record for record in file_records(source) if record['path'] != 'inventory.json']
    if value['files'] != records:
        raise ValueError('Corresponding Source file hashes do not match inventory')
    if len(value['components']) != len(manifest['sources']):
        raise ValueError('Incomplete source inventory')
    for component, pin in zip(value['components'], manifest['sources']):
        if (component['name'] != pin['name'] or component['version'] != pin['version']
                or component['url'] != pin['url']
                or component['sources'] != [{'path': 'sources/' + pin['archive'], 'sha256': pin['sha256']}]):
            raise ValueError('Component source pin does not match inventory')
        for kind in ('sources', 'recipes', 'licenses'):
            expected = [record for record in records if (
                record['path'] == 'sources/' + pin['archive'] if kind == 'sources'
                else record['path'].startswith('recipes/' if kind == 'recipes' else 'licenses/' + pin['name'] + '/')
            )]
            if not expected or component[kind] != expected:
                raise ValueError(f'Incomplete or invalid {kind}: {pin["name"]}')
    for item in current_inputs:
        if digest(source / 'recipes' / item['path']) != item['sha256']:
            raise ValueError('Preserved recipe differs from the current recipe')
    for pin in manifest['sources']:
        if digest(source / 'sources' / pin['archive']) != pin['sha256']:
            raise ValueError('Preserved source archive differs from the source pin')
    return source


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=('prepare', 'audit', 'inventory', 'verify', 'no-external-deps'))
    parser.add_argument('--repo', type=Path)
    parser.add_argument('--deps', type=Path)
    args, extra = parser.parse_known_args()
    if args.action == 'no-external-deps':
        return 1
    if extra or args.repo is None or args.deps is None:
        parser.error('--repo and --deps are required; no extra arguments allowed')
    if args.action == 'audit':
        print(audit_graph(args.deps))
    else:
        result = {'prepare': prepare, 'inventory': inventory, 'verify': verify_inventory}[args.action](args.repo, args.deps)
        if result is not None:
            print(result)
    return 0


if __name__ == '__main__':
    sys.exit(main())
