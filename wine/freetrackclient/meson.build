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

dll = shared_library(
  'FreeTrackClient',
  'freetrackclient.c',
  res,
  name_prefix: '',
  dependencies: version_dep,
)

test_exe = executable('freetrackclient_test', 'freetrackclient_test.c')

test(
  'freetrackclient test',
  python3,
  args: [files('freetrackclient_test.py'), dll, test_exe],
)
