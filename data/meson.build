desktop_conf = configuration_data()
desktop_conf.set('app-id', app_id)

desktop_file = i18n.merge_file(
        input: configure_file(
      		        input: 'org.gnome.gitlab.wwarner.Solitaire.desktop.in',
      		       output: '@BASENAME@',
      		configuration: desktop_conf
               ),
       output: '@0@.desktop'.format(app_id),
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_conf = configuration_data()
appstream_conf.set('app-id', app_id)

appstream_file = i18n.merge_file(
        input: configure_file(
        	 	 input: 'org.gnome.gitlab.wwarner.Solitaire.metainfo.xml.in',
        		output: '@BASENAME@',
         	 configuration: appstream_conf
        	),
       output: '@0@.appdata.xml'.format(app_id),
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

gschema_conf = configuration_data()
gschema_conf.set('app-id', app_id)

configure_file(
  input: 'org.gnome.gitlab.wwarner.Solitaire.gschema.xml',
  output: '@0@.gschema.xml'.format(app_id),
  install: true,
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas',
  configuration: gschema_conf
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])


service_conf = configuration_data()
service_conf.set('bindir', get_option('prefix') / get_option('bindir'))
service_conf.set('app-id', app_id)

configure_file(
  input: 'org.gnome.gitlab.wwarner.Solitaire.service.in',
  output: '@0@.service'.format(app_id),
  install: true,
  configuration: service_conf,
  install_dir: get_option('datadir') / 'dbus-1' / 'services'
)

subdir('icons')
