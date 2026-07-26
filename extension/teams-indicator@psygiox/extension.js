// extension.js
// Опциональное расширение GNOME Shell: показывает бейдж непрочитанных
// сообщений Teams прямо в верхней панели (рядом с часами), как
// альтернатива/дополнение к иконке в системном трее через appindicator.
//
// Механизм простой и без DBus-сервиса: приложение teams-linux пишет
// текущее число непрочитанных в файл
//   ~/.local/share/teams-linux/unread-count
// а расширение раз в 3 секунды перечитывает этот файл (GFileMonitor
// используется как основной путь, polling — как фолбэк).

import St from 'gi://St';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import Main from 'resource:///org/gnome/shell/ui/main.js';

const UNREAD_FILE = GLib.build_filenamev([
  GLib.get_home_dir(),
  '.local', 'share', 'teams-linux', 'unread-count',
]);

const TeamsIndicator = class extends PanelMenu.Button {
  static {
    // GObject.registerClass выполняется автоматически при экспорте
    // через imports в новых версиях GJS/Shell 45+, оставляем как есть.
  }

  _init() {
    super._init(0.0, 'Teams Unread Indicator');

    this._label = new St.Label({
      text: '',
      y_align: 2 /* Clutter.ActorAlign.CENTER */,
      style_class: 'teams-indicator-label',
    });
    this.add_child(this._label);
    this.hide();

    this._monitor = null;
    this._timeoutId = null;
    this._setupMonitor();
    this._refresh();
  }

  _setupMonitor() {
    try {
      const file = Gio.File.new_for_path(UNREAD_FILE);
      this._monitor = file.monitor(Gio.FileMonitorFlags.NONE, null);
      this._monitor.connect('changed', () => this._refresh());
    } catch (e) {
      logError(e, 'teams-indicator: не удалось создать GFileMonitor, использую polling');
    }
    // Полинг как страховка на случай, если inotify не сработал (сетевые ФС и т.п.)
    this._timeoutId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 3, () => {
      this._refresh();
      return GLib.SOURCE_CONTINUE;
    });
  }

  _refresh() {
    let count = 0;
    try {
      const [ok, contents] = GLib.file_get_contents(UNREAD_FILE);
      if (ok) {
        count = parseInt(new TextDecoder().decode(contents).trim(), 10) || 0;
      }
    } catch (_e) {
      count = 0;
    }

    if (count > 0) {
      this._label.set_text(count > 99 ? '99+' : String(count));
      this.show();
    } else {
      this.hide();
    }
  }

  destroy() {
    if (this._timeoutId) {
      GLib.source_remove(this._timeoutId);
      this._timeoutId = null;
    }
    if (this._monitor) {
      this._monitor.cancel();
      this._monitor = null;
    }
    super.destroy();
  }
};

export default class TeamsIndicatorExtension extends Extension {
  enable() {
    this._indicator = new TeamsIndicator();
    this._indicator.name = 'teams-indicator';
    // Добавляем в правый угол панели, рядом с системными иконками
    Main.panel.addToStatusArea('teams-indicator', this._indicator, 1, 'right');
  }

  disable() {
    this._indicator?.destroy();
    this._indicator = null;
  }
}
