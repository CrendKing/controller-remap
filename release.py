if __name__ == '__main__':
    from pathlib import Path
    import os
    import subprocess

    app_name = Path(__file__).parent.name
    subprocess.run(['taskkill', '/F', '/IM', f'{app_name}.exe'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    for build_type in ('debug', 'release'):
        target_dir = Path(__file__).with_name('target') / build_type
        target_dir.mkdir(parents=True, exist_ok=True)

        config_path = Path(__file__).with_name('config') / f'{app_name}.toml'
        config_link_target = target_dir / f'{app_name}.toml'

        if not config_link_target.exists():
            os.link(config_path, config_link_target)

    subprocess.check_call(['cargo', 'build', '--release'])
