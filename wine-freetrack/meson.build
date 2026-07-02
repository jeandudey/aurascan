project(
  'wine-freetrack',
  'c',
  version: '0.1.0',
)

is_windows = target_machine.system() == 'windows'
arch_dir_prefix = target_machine.cpu_family() == 'x86_64' ? 'x86_64' : 'i386'
os_dir_suffix = is_windows ? 'windows' : 'unix'
install_dir = get_option('libdir') / 'wine' / arch_dir_prefix + '-' + os_dir_suffix

if not is_windows
  subdir('posixbridge')
endif

subdir('freetrackclient')
subdir('npclient')
