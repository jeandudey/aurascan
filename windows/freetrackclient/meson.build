project('freetrackclient', 'c')

if host_machine.cpu_family() == 'x86_64'
  freetrackclient_name = 'freetrackclient64'
  npclient_name = 'NPClient64'
elif host_machine.cpu_family() == 'x86'
  freetrackclient_name = 'freetrackclient'
  npclient_name = 'NPClient'
endif

shared_library(freetrackclient_name, 'freetrackclient.c', name_prefix: '')
shared_library(npclient_name, 'npclient.c', name_prefix: '')
