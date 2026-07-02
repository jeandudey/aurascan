if is_windows
  shared_library(
    'NPClient',
    'npclient.c',
    include_directories: '../include',
    name_prefix: '',
    link_depends: 'npclient.spec',
    link_args: ['-Wl,--wine-builtin', meson.current_source_dir() / 'npclient.spec'],
    install: true,
    install_dir: install_dir,
  )
else
endif
