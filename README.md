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
- [Исправленные баги (для истории)](#исправленные-баги-для-истории)
- [Диагностика звонков и создания встречи](#диагностика-звонков-и-создания-встречи)
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
│   ├── session.rs              # права 0700 на каталог данных, очистка сессии
│   └── media.rs                 # автоматическое разрешение камеры/микрофона (звонки)
├── injected/
│   ├── teams-bridge.js         # JS, инжектируемый в веб-страницу Teams
│   ├── user-agent-data-shim.js # шим navigator.userAgentData (см. раздел "Диагностика звонков")
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

- **Ограничение навигации.** Окно грузит только `teams.microsoft.com/v2`
  (плюс allowlist доменов Microsoft в `src/security.rs` — логин, cloud.microsoft
  и т.д.). Если top-level навигация попытается уйти на посторонний домен
  (например, ссылка из чата), она блокируется в двух независимых слоях:
  `capabilities/default.json > remote.urls` отзывает доступ к
  `window.__TAURI__` вне allowlist на уровне рантайма Tauri, а
  `on_navigation` в `src/main.rs` физически не даёт окну туда перейти.
  Вместо тихого редиректа во внешний браузер показывается **модалка с
  адресом ссылки** и явным выбором «Открыть в браузере» / «Отмена» — так
  видно, куда именно ведёт ссылка, прежде чем она откроется.
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

## Исправленные баги (для истории)

Три проблемы, которые были в первых сборках, и что их вызывало:

**Звонки и создание видео-встречи не работали.** Причины оказались три:
1. По [официальной документации Microsoft](https://learn.microsoft.com/en-us/microsoftteams/unsupported-browsers)
   Teams урезает калькинг-функции в браузерах, которые определяются как
   Safari — а исходный User-Agent (`Version/17.0 Safari/605.1.15`) как раз
   выдавал webkit2gtk за Safari. UA заменён на Chrome-подобный
   (`src/main.rs`), чтобы Teams включал полный набор функций звонков.
2. WebKitGTK [по документации](https://webkitgtk.org/reference/webkit2gtk/2.14.6/WebKitUserMediaPermissionRequest.html)
   отклоняет запрос доступа к камере/микрофону (`getUserMedia`) **по
   умолчанию**, если его не обработать явно — Tauri/wry не делают этого
   автоматически ([tauri-apps/wry#81](https://github.com/tauri-apps/wry/issues/81)).
   `src/media.rs` вешает обработчик `permission-request` на сырой
   webkit2gtk-хэндл и разрешает такие запросы.
3. Разрешить permission-запрос — не то же самое, что включить сам
   WebRTC-стек: `enable-webrtc`/`enable-media-stream` и смежные
   WebKitSettings-флаги Tauri/wry **не включают по умолчанию**, официальная
   поддержка WebRTC в Tauri всё ещё не закончена (подробный разбор с рабочим
   воркэраундом: [tauri-apps/discussions#8426](https://github.com/orgs/tauri-apps/discussions/8426)).
   `src/media.rs` теперь явно включает `enable_webrtc`, `enable_media_stream`,
   `enable_mediasource` и связанные настройки. Плюс сам WebRTC в webkit2gtk
   технически работает через GStreamer (`webrtcbin` из `gst-plugins-bad`) —
   без системных пакетов `gstreamer1.0-plugins-bad`/`gstreamer1.0-nice`
   (или их аналогов на Fedora/openSUSE) звонок не соберёт медиапоток, даже
   если все настройки правильные. Пакеты добавлены в `build-deb.sh` /
   `build-rpm.sh` и в runtime-зависимости `.deb`/`.rpm`.

   **Честно про границы:** по опыту сообщества полноценное видео в
   webkit2gtk на **Wayland** местами упирается в ошибки декодирования
   буфера (`GBM-DRV error`) на некоторых связках GPU-драйвер/GStreamer — это
   ограничение самого WebKitGTK, а не этого проекта. Если после сборки звук
   в звонке работает, а видео — нет, попробуйте для диагностики
   `GDK_BACKEND=x11 teams-linux` (нужен установленный XWayland).

**Разлогинивало при закрытии через трей/крестик.** webkit2gtk пишет куки и
localStorage на диск асинхронно через GLib main loop; при мгновенном
`app.exit(0)` (или уничтожении окна) он не успевал это сделать. Теперь:
крестик окна прячет его в трей вместо закрытия (`on_window_event` +
`prevent_close()` в `src/main.rs`), а реальный выход из трея
(`graceful_quit` в `src/tray.rs`) сначала прячет окно и ждёт короткую паузу
перед завершением процесса, давая время на запись сессии.

**После авторизации ссылка открывалась в браузере.** С 2023–2025 Microsoft
последовательно переводит Teams и другие M365-приложения на новый домен
[`cloud.microsoft`](https://techcommunity.microsoft.com/blog/microsoft_365blog/introducing-cloud-microsoft-a-unified-domain-for-microsoft-365-apps-and-services/3804961)
(`teams.cloud.microsoft` уже в проде и Teams сам туда редиректит после
логина). Этого домена — как и линк-шортенеров `aka.ms`/`1drv.ms` и
отдельных доменов вроде `microsoftstream.com` — не было в allowlist
(`src/security.rs`), поэтому легитимный редирект самого Teams считался
"чужим доменом" и улетал во внешний браузер. Список расширен.


## Диагностика звонков и создания встречи

Звонок/видеозвонок/создание встречи — самая нестабильная часть проекта, и
вот что удалось выяснить и исправить, а что осталось открытым вопросом.

**Что сделано в этой итерации, по мотивам [teams-for-linux](https://github.com/IsmaelMartinez/teams-for-linux)**
(зрелый, рабочий Electron-обёртка над тем же Teams v2 — стоит посмотреть на
их подход, если что-то из наших фиксов не сработает):

- **X11 вместо нативного Wayland по умолчанию.** В [документации
  teams-for-linux](https://ismaelmartinez.github.io/teams-for-linux/configuration)
  прямо сказано: они **по умолчанию всегда** запускаются как
  `--ozone-platform=x11`, то есть через XWayland, а не нативный Wayland —
  именно из-за проблем с камерой/видеопотоком на Wayland. Это тот же
  `GBM-DRV error`, что мы нашли раньше в контексте webkit2gtk. Применили
  тот же подход: `.desktop`-файлы теперь запускают приложение с
  `GDK_BACKEND=x11` (нужен установленный XWayland — на большинстве
  дистрибутивов с GNOME он уже стоит по умолчанию).
- **Инструменты разработчика в трее.** Добавлен пункт «Открыть инструменты
  разработчика» — открывает WebKit Inspector поверх окна Teams. Именно так
  разработчик teams-for-linux диагностировал недавнюю поломку calling-стека
  (см. ниже) — через вкладку Console/Network в момент клика по кнопке
  звонка. Если звонок не запускается — открой инспектор, нажми на кнопку
  звонка и посмотри на ошибки в Console; это единственный надёжный способ
  узнать, что происходит на самом деле, вместо гадания.
- **Шим `navigator.userAgentData` (User-Agent Client Hints).** Найденная
  причина disabled-кнопки видеозвонка/недоступной кнопки аудиозвонка: наш
  поддельный User-Agent говорит "я Chrome/126", но
  [`navigator.userAgentData` — это чисто Chromium-фича](https://developer.mozilla.org/en-US/docs/Web/API/User-Agent_Client_Hints_API),
  которую WebKit не реализует вовсе (ни Safari, ни webkit2gtk). Получается
  явное противоречие: UA-строка утверждает одно, а более надёжный (и
  менее подделываемый) сигнал Client Hints отсутствует — чего у настоящего
  Chrome не бывает никогда. Похоже, именно по этому расхождению Teams
  отключает калькинг-кнопки. Добавлен `injected/user-agent-data-shim.js`,
  который подставляет правдоподобный объект `navigator.userAgentData` с
  той же версией Chrome (126), что и в UA-строке. Важно: подключён через
  `initialization_script` (document-start), а не через `on_page_load` — он
  обязан выполниться раньше первого же скрипта самого Teams, иначе
  проверка совместимости браузера уже пройдёт мимо него.

**Честно про открытый вопрос.** Пока разбирался с этим багом, нашёл, что у
самого teams-for-linux **прямо сейчас** (issue [#2523](https://github.com/IsmaelMartinez/teams-for-linux/issues/2523),
открыт 11 мая 2026) сломана демонстрация экрана в звонках, а рядом
упомянуты смежные свежие проблемы calling-стека Teams v2
([#2453](https://github.com/IsmaelMartinez/teams-for-linux/issues/2453),
[#2454](https://github.com/IsmaelMartinez/teams-for-linux/issues/2454)).
Внутри debug-трейса видно, что калькинг-стек Teams (файлы вида
`calling-pluginless-*.js`) в определённых сценариях сам пишет в консоль
`No start call scenario` и `Failed getDevices due to no deviceDiscovery
module found`, ещё до того, как WebRTC вообще пытается получить доступ к
камере/микрофону. Это значит, что часть проблемы может быть не в
webkit2gtk и не в этом проекте, а в самом клиентском JS-коде Teams v2,
который в отдельных сценариях (тип браузера, тип входа) отказывается
инициализировать звонок сам, ещё до getUserMedia. Если после сборки с
X11-фиксом и включённым WebRTC-стеком (`src/media.rs`) кнопка звонка
всё ещё не работает — открой DevTools (пункт трея) и посмотри вкладку
Console в момент клика: если там видно `calling-pluginless` с похожими
сообщениями — это, по всей видимости, вопрос к самому Teams v2, а не к
обёртке, и стоит проверить тот же сценарий в `https://teams.microsoft.com`
через настоящий Chrome для сравнения.


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
