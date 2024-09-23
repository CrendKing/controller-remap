if __name__ == '__main__':
    from pathlib import Path
    import os
    import subprocess
    import sys

    app_name = Path(__file__).parent.name
    subprocess.run(('taskkill', '/F', '/IM', f'{app_name}.exe'), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    out_dir = Path(__file__).with_name('out') / 'release'
    config_path = Path(__file__).with_name('config') / f'{app_name}.toml'
    config_link_target = out_dir / f'{app_name}.toml'

    if not config_link_target.exists():
        os.link(config_path, config_link_target)

    sys.path.insert(1, str(Path(__file__).parents[1]))
    import Script.cargo_build
    Script.cargo_build.process(['cargo', 'build', '--release'])
