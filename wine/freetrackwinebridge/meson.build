project('freetrackwinebridge', 'c', version: '0.1.0')

arch_dir_prefix = target_machine.cpu_family() == 'x86_64' ? 'x86_64' : 'i386'
bindir = get_option('libdir') / 'wine' / arch_dir_prefix + '-unix'

executable(
  meson.project_name(),
  'main.c',
  'posix.c',
  name_suffix: 'exe.so',
  install: true,
  install_dir: bindir,
)
