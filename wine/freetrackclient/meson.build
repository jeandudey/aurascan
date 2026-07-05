project('freetrackclient', 'c', version: '1.0.0.0')

windows = import('windows')

cc = meson.get_compiler('c')

python3 = find_program('python3')
version_dep = cc.find_library('version')

conf = configuration_data({ 'dllversion': meson.project_version() })
freetrackclient_rc = configure_file(
  input: 'freetrackclient.rc.in',
  output: 'freetrackclient.rc',
  configuration: conf,
)
res = windows.compile_resources(freetrackclient_rc)

name = 'FreeTrackClient'
if host_machine.cpu_family() == 'x86_64'
  name += '64'
endif

dll = shared_library(
  name,
  'freetrackclient.c',
  res,
  name_prefix: '',
  dependencies: version_dep,
  install: true,
)

test_exe = executable('freetrackclient_test', 'freetrackclient_test.c')

test(
  'freetrackclient test',
  python3,
  args: [files('freetrackclient_test.py'), dll, test_exe],
)

ui_res = windows.compile_resources('freetrackclient_test_ui.rc')
executable(
  'freetrackclient_test_ui',
  'freetrackclient_test_ui.c',
  ui_res,
  install: true,
)
