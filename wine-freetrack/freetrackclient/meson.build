if is_windows
  shared_library(
    'freetrackclient',
    'freetrackclient.c',
    include_directories: '../include',
    name_prefix: '',
    link_depends: 'freetrackclient.spec',
    link_args: ['-Wl,--wine-builtin', meson.current_source_dir() / 'freetrackclient.spec'],
    install: true,
    install_dir: install_dir,
  )

  executable(
    'loadtest',
    'loadtest.c',
    include_directories: '../include',
    install: false,
  )
else
  shared_library(
    'freetrackclient',
    'unixlib.c',
    include_directories: '../include',
    name_prefix: '',
    link_with: posixbridge,
    install: true,
    install_dir: install_dir,
  )
endif
