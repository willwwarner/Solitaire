project('solitaire', ['c', 'rust'],
          version: '50.alpha',
    meson_version: '>= 1.0.0',
  default_options: [ 'warning_level=2', 'werror=false', ],
)

i18n = import('i18n')
gnome = import('gnome')

cargo_opt  = [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / 'src' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
  app_id = 'org.gnome.gitlab.wwarner.Solitaire'
else
  rust_target = 'debug'
  app_id = 'org.gnome.gitlab.wwarner.Solitaire.Devel'
endif

glib_dependency = dependency('glib-2.0', version: '>= 2.80.0')
gtk_dependency = dependency('gtk4', version: '>= 4.16.0')
adw_dependency = dependency('libadwaita-1', version: '>= 1.8')
libgnome_games_support = subproject ('libgnome-games-support',
                                     default_options: 'superproject_name=solitaire')

libgnome_games_support_dep = libgnome_games_support.get_variable ('lggs_dependency')

subdir('data')
subdir('src')
subdir('po')

gnome.post_install(
     glib_compile_schemas: true,
    gtk_update_icon_cache: true,
  update_desktop_database: true,
)
