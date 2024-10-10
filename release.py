if __name__ == '__main__':
    from pathlib import Path
    import os
    import subprocess

    app_name = Path(__file__).parent.name
    subprocess.run(['taskkill', '/F', '/IM', f'{app_name}.exe'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    for build_type in ('debug', 'release'):
        target_dir = Path(__file__).with_name('target') / build_type
        target_dir.mkdir(parents=True, exist_ok=True)

        source_config = Path(__file__).with_name('config') / f'{app_name}.toml'
        target_config = target_dir / f'{app_name}.toml'

        if target_config.exists() and target_config.stat().st_nlink == 1:
            target_config.unlink()
        if not target_config.exists():
            os.link(source_config, target_config)

    subprocess.check_call(['cargo', 'build', '--release'])
