<div align="center">

# Teams Linux

**Неофициальный клиент Microsoft Teams для Debian/GNOME на Tauri v2**

Сделан для себя, потому что официального клиента под Linux у Microsoft просто нет,
а голая вкладка браузера не даёт нормальных уведомлений, трея и хоткеев.

[![Platform](https://img.shields.io/badge/platform-Linux-informational)](#)
[![Made with Tauri](https://img.shields.io/badge/made%20with-Tauri%20v2-24C8DB)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](#лицензия)
[![Status](https://img.shields.io/badge/status-личный%20проект-orange)](#это-личный-проект-если-что)

</div>

---

## Зачем это существует

У Microsoft Teams нет нативного клиента под Linux — есть только веб-версия
`teams.microsoft.com/v2`, которую приходится открывать вкладкой в браузере.
Работает, но:

- уведомления — куцые браузерные, без экшенов "Ответить"/"Прочитано"
- нет иконки в трее и счётчика непрочитанных
- нет глобальных хоткеев для mute/камеры, когда окно не в фокусе
- интерфейс не подхватывает системную тему GNOME
- вкладка теряется среди остальных 30 открытых

Этот проект — тонкая нативная обёртка на **Tauri v2 (Rust + webkit2gtk)**
поверх той же самой `teams.microsoft.com/v2`, с интеграцией в GNOME на
уровне DBus/gsettings. Никакого реверс-инжиниринга протокола Teams, никакого
кастомного бэкенда — просто нормальная десктопная оболочка вокруг официальной
веб-версии.

## Возможности

- 🔔 **Нативные уведомления** через `org.freedesktop.Notifications` (DBus) —
  с кнопками «Ответить» и «Прочитано» прямо в Notification Center GNOME,
  а не куцые webkit-уведомления
- 🗂️ **Трей** (libayatana-appindicator) с меню и счётчиком непрочитанных +
  опциональное **расширение GNOME Shell** для счётчика в топ-баре
- ⌨️ **Глобальные хоткеи** `Super+Shift+M` (mute) и `Super+Shift+O` (камера) —
  работают даже когда окно свёрнуто
- 🎨 **Синхронизация темы** со светлой/тёмной темой GNOME на лету
- 🌙 **Синхронизация Do Not Disturb** с системным DND GNOME
- 🔗 Автозапуск, единственный экземпляр приложения, deep links `msteams://`
- 📦 Нативная сборка в **`.deb`** и **`.rpm`** одной командой

## Скриншоты

> TODO: добавить скриншоты после первого релиза — окно с тёмной темой,
> нативное уведомление в Notification Center, меню трея.

## Быстрый старт

### Debian / Ubuntu

```bash
git clone https://github.com/PsyGioX/teams-linux-tauri.git
cd teams-linux-tauri
chmod +x packaging/build-deb.sh
./packaging/build-deb.sh
sudo apt install ./target/release/bundle/deb/Teams-Linux_0.1.0_amd64.deb
```

### Fedora / RHEL / openSUSE

```bash
git clone https://github.com/PsyGioX/teams-linux-tauri.git
cd teams-linux-tauri
chmod +x packaging/build-rpm.sh
./packaging/build-rpm.sh
sudo dnf install ./target/release/bundle/rpm/Teams-Linux-0.1.0-1.x86_64.rpm
```

Оба скрипта сами ставят системные зависимости (webkit2gtk, GTK, Rust/Tauri
CLI при отсутствии) и собирают пакет — руками почти ничего делать не нужно.

## Оглавление

- [Структура проекта](#структура-проекта)
- [Зависимости и сборка вручную](#зависимости-и-сборка-вручную)
- [Разработка (`cargo tauri dev`)](#разработка)
- [Автозапуск](#автозапуск)
- [Расширение GNOME Shell](#расширение-gnome-shell)
- [Глобальные хоткеи](#глобальные-хоткеи)
- [Безопасность](#безопасность)
- [Известные проблемы](#известные-проблемы-и-ограничения)
- [Это личный проект, если что](#это-личный-проект-если-что)
- [Лицензия](#лицензия)

---

## Структура проекта

```
teams-linux-tauri/
├── Cargo.toml                  # зависимости и метаданные Rust/Tauri
├── build.rs                    # build-скрипт tauri-build
├── tauri.conf.json             # конфигурация Tauri v2 (бандл, иконки, deb/rpm)
├── capabilities/default.json   # права доступа окна + allowlist доменов (remote.urls)
├── src/
│   ├── main.rs                 # точка входа, окно, on_navigation, регистрация плагинов
│   ├── notifications.rs        # мост Web Notification API -> DBus
│   ├── theme.rs                # синхронизация темы GNOME
│   ├── dnd.rs                  # синхронизация Do Not Disturb
│   ├── tray.rs                 # трей, меню, счётчик непрочитанных, "Выйти и очистить сессию"
│   ├── shortcuts.rs            # глобальные хоткеи mute/camera
│   ├── security.rs             # allowlist доменов Microsoft для on_navigation
│   └── session.rs              # права 0700 на каталог данных, очистка сессии
├── injected/
│   ├── teams-bridge.js         # JS, инжектируемый в веб-страницу Teams
│   └── index.html              # заглушка (Tauri требует frontendDist-папку)
├── icons/                      # иконки приложения и трея
├── packaging/
│   ├── teams-linux.desktop           # .desktop записи приложения
│   ├── teams-linux-autostart.desktop # .desktop для автозапуска
│   ├── build-deb.sh                  # авто-установка зависимостей + сборка .deb
│   └── build-rpm.sh                  # то же для Fedora/RHEL/openSUSE (.rpm)
└── extension/teams-indicator@psygiox/  # опциональное расширение GNOME Shell
    ├── metadata.json
    └── extension.js
```

## Зависимости и сборка вручную

Если не хочется гонять готовый скрипт — вот что нужно на Debian:

```bash
sudo apt install -y \
  build-essential curl wget file git \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
  libayatana-appindicator3-dev libssl-dev libnotify-dev \
  patchelf pkg-config

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo install tauri-cli --version "^2"

cargo tauri build --bundles deb   # или: --bundles rpm
```

> Требуется Rust **≥ 1.80** (используется `std::sync::LazyLock` в
> `src/notifications.rs`). Актуальный rustup ставит подходящую версию сам.

Для Fedora/openSUSE — те же пакеты называются иначе, см.
[`packaging/build-rpm.sh`](packaging/build-rpm.sh), он ставит их автоматически.

## Разработка

```bash
cargo tauri dev
```

Откроется окно с Teams v2 и уже подключённым JS-мостом
(`injected/teams-bridge.js`) — удобно для отладки уведомлений, хоткеев,
темы, без пересборки пакета на каждое изменение.

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 cargo tauri dev   # если рендер тормозит на NVIDIA
GDK_BACKEND=wayland cargo tauri dev                 # форсировать Wayland-бэкенд GTK
```

## Автозапуск

```bash
mkdir -p ~/.config/autostart
cp packaging/teams-linux-autostart.desktop ~/.config/autostart/
```

Либо программно через встроенный `tauri-plugin-autostart`, если добавите
переключатель в UI:

```js
import { enable } from '@tauri-apps/plugin-autostart';
await enable();
```

## Расширение GNOME Shell

Дублирует счётчик непрочитанных из трея прямо в верхнюю панель:

```bash
mkdir -p ~/.local/share/gnome-shell/extensions/teams-indicator@psygiox
cp -r extension/teams-indicator@psygiox/* \
   ~/.local/share/gnome-shell/extensions/teams-indicator@psygiox/

# X11: Alt+F2 -> r -> Enter; Wayland: перелогиниться
gnome-extensions enable teams-indicator@psygiox
```

## Глобальные хоткеи

| Комбинация      | Действие                |
|-----------------|--------------------------|
| `Super+Shift+M` | Mute / Unmute микрофон  |
| `Super+Shift+O` | Вкл/выкл камера          |

Конфликтует с уже занятой системной комбинацией GNOME — меняется в
`src/shortcuts.rs`.

## Безопасность

Кратко (без прикрас) — что учтено и что нет:

- **Ограничение навигации.** Окно грузит только `teams.microsoft.com/v2`.
  Если top-level навигация попытается уйти на посторонний домен (например,
  фишинговая ссылка из чата), она блокируется в двух независимых слоях:
  `capabilities/default.json > remote.urls` отзывает доступ к
  `window.__TAURI__` вне allowlist доменов Microsoft на уровне рантайма
  Tauri, а `on_navigation` в `src/main.rs` физически не даёт окну туда
  перейти — ссылка открывается в системном браузере вместо этого.
- **Локальная сессия НЕ шифруется** приложением сверх того, что даёт
  webkit2gtk из коробки — как и любой браузерный профиль без FDE. Реальная
  защита от кражи куки/токенов при физическом доступе к машине —
  **полнодисковое шифрование (LUKS)** на уровне ОС, не на уровне
  приложения. Из практичного — есть пункт трея **«Выйти и очистить
  сессию»**, который стирает локальный профиль.
- **DOM-селекторы** для mute/camera-хоткеев и счётчика непрочитанных
  специфичны для текущей разметки Teams v2 и могут сломаться при
  редизайне от Microsoft. Сделаны с fallback-вариантами и предупреждают
  (console.warn + нативное уведомление), а не молчат.
- Пакет **не аффилирован с Microsoft** — независимая обёртка над публичной
  веб-версией, доступной в любом браузере по той же ссылке.

## Известные проблемы и ограничения

- **Screen sharing на Wayland** зависит от `xdg-desktop-portal` + PipeWire:
  ```bash
  sudo apt install xdg-desktop-portal-gnome
  systemctl --user status xdg-desktop-portal
  ```
- Селекторы в `injected/teams-bridge.js` могут потребовать обновления после
  редизайна интерфейса Teams — issues и PR приветствуются.
- Тестировалось на Debian Sid/GNOME и Fedora, на других DE (KDE, XFCE) трей
  и уведомления должны работать через стандартные freedesktop-протоколы, но
  специально не проверялось.

## Это личный проект, если что

Собрал это в первую очередь под свои задачи — GNOME-first, никаких
компромиссов ради поддержки других DE или Windows. Если тебе для работы
нужен именно Teams под Linux и раздражает то же самое, что и меня — welcome,
issues и PR приветствуются. Просто не жди enterprise-уровня поддержки и
регулярных релизов по расписанию: чиню, когда сам натыкаюсь на поломку или
кто-то присылает внятный баг-репорт.

Автор: [PsyGioX](https://github.com/PsyGioX)

## Лицензия

MIT — делайте с кодом что хотите. Сам Teams остаётся проприетарным сервисом
Microsoft, этот проект только упаковывает его публичную веб-версию в
нативную оболочку.