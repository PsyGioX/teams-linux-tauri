// teams-bridge.js
// Инжектится в webview Teams при загрузке страницы (initializationScript в tauri.conf
// либо через window.eval при событии 'did-finish-load'). Отвечает за:
//   1. Перехват window.Notification -> нативные DBus-уведомления GNOME
//   2. Реакцию на действия из уведомлений (открыть чат / отметить прочитанным)
//   3. Применение CSS-темы GNOME (light/dark) и синхронизацию DND
//   4. Обработку глобальных хоткеев mute/camera кликом по кнопкам Teams
//   5. Отправку счётчика непрочитанных в трей

(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  // ---------- Утилита: предупреждать один раз за сессию, а не молчать ----------
  // Главная проблема хрупких DOM-селекторов не в том, что они ломаются
  // (это неизбежно при любом редизайне Teams), а в том, что раньше это
  // происходило БЕСШУМНО. Теперь при первом промахе — заметный console.warn
  // + нативное уведомление, чтобы было видно "фича сломалась", а не тишина.
  const warnedOnce = new Set();
  function warnSelectorMiss(feature, details) {
    if (warnedOnce.has(feature)) return;
    warnedOnce.add(feature);
    console.warn(
      `[teams-linux] Не найден элемент интерфейса для "${feature}" (${details}). ` +
        `Скорее всего Microsoft обновил разметку Teams — функция временно недоступна.`
    );
    invoke("send_native_notification", {
      payload: {
        title: "Teams Linux: интерфейс мог обновиться",
        body: `Функция "${feature}" не нашла нужный элемент на странице. Сообщите в github issue, если повторяется.`,
        conversation_id: null,
        allow_reply: false,
      },
    }).catch(() => {});
  }

  // Пробует по очереди несколько CSS-селекторов, возвращает первый найденный элемент
  function queryFirst(selectors) {
    for (const sel of selectors) {
      const el = document.querySelector(sel);
      if (el) return el;
    }
    return null;
  }

  // ---------- 1. Перехват Web Notification API ----------
  const NativeNotification = window.Notification;
  function PatchedNotification(title, options = {}) {
    const conversationId = extractConversationIdFromDom();
    invoke("send_native_notification", {
      payload: {
        title,
        body: options.body || "",
        conversation_id: conversationId,
        allow_reply: true,
      },
    }).catch((e) => console.error("[teams-linux] notify error", e));

    // Возвращаем фейковый объект, совместимый с ожидаемым API Notification
    return { close() {}, onclick: null, onclose: null, onerror: null, onshow: null };
  }
  PatchedNotification.permission = "granted";
  PatchedNotification.requestPermission = (cb) => {
    if (cb) cb("granted");
    return Promise.resolve("granted");
  };
  window.Notification = PatchedNotification;

  function extractConversationIdFromDom() {
    // Teams v2 хранит текущий чат в URL хэше вида #/conversations/<id>.
    // Это самый устойчивый источник (URL меняется реже разметки), поэтому
    // никакого DOM-fallback тут не нужно — просто возвращаем null, если
    // формат URL когда-нибудь изменится, вместо попытки угадать по DOM.
    const match = location.hash.match(/conversations\/([^/?]+)/);
    return match ? match[1] : null;
  }

  // ---------- 2. Действия из нативных уведомлений ----------
  listen("teams-linux://notification-action", (event) => {
    const { action, conversation_id } = event.payload;
    if (action === "default" || action === "reply") {
      if (conversation_id) {
        location.hash = `/conversations/${conversation_id}`;
      }
      window.focus();
    }
    if (action === "mark-read" && conversation_id) {
      // Место для вызова внутреннего API Teams пометки прочитанным,
      // если потребуется — через клик по DOM-элементу непрочитанного чата.
      const el = document.querySelector(`[data-convid="${conversation_id}"]`);
      el?.click();
    }
  });

  // ---------- 3. Тема GNOME ----------
  function applyTheme(theme) {
    document.documentElement.dataset.gnomeTheme = theme;
    const styleId = "teams-linux-gnome-css";
    let style = document.getElementById(styleId);
    if (!style) {
      style = document.createElement("style");
      style.id = styleId;
      document.head.appendChild(style);
    }
    style.textContent =
      theme === "Dark"
        ? `:root { color-scheme: dark; } body { background: #1e1e1e !important; }`
        : `:root { color-scheme: light; }`;
  }
  invoke("get_system_theme").then(applyTheme);
  listen("teams-linux://theme-changed", (e) => applyTheme(e.payload));

  // ---------- 4. Do Not Disturb ----------
  listen("teams-linux://dnd-changed", (e) => {
    console.log("[teams-linux] GNOME DND:", e.payload.enabled);
    // Здесь можно программно кликнуть по селектору статуса Teams и выставить
    // "Не беспокоить" — селектор специфичен для текущей версии Teams v2 UI
    // и должен обновляться при обновлениях интерфейса Microsoft.
  });

  // ---------- 5. Глобальные хоткеи mute/camera ----------
  // aria-label у Teams бывает на языке интерфейса пользователя, поэтому
  // проверяем сразу несколько вариантов (en/ru), а не один жёстко зашитый.
  function clickByAriaLabelContains(keywords, feature) {
    const btn = Array.from(document.querySelectorAll("button")).find((b) => {
      const label = (b.getAttribute("aria-label") || "").toLowerCase();
      return keywords.some((kw) => label.includes(kw));
    });
    if (!btn) {
      warnSelectorMiss(feature, `искали кнопку с aria-label из [${keywords.join(", ")}]`);
      return;
    }
    btn.click();
  }
  listen("teams-linux://hotkey-toggle-mute", () =>
    clickByAriaLabelContains(["microphone", "mute", "микрофон"], "переключение микрофона")
  );
  listen("teams-linux://hotkey-toggle-camera", () =>
    clickByAriaLabelContains(["camera", "видео", "камера"], "переключение камеры")
  );

  // ---------- 6. Счётчик непрочитанных в трей ----------
  const { emit } = window.__TAURI__.event;
  let lastReportedCount = -1;
  function reportUnreadCount() {
    // Несколько вариантов селектора: Teams периодически меняет data-tid
    // и CSS-классы между релизами Fluent UI.
    const badge = queryFirst([
      '[data-tid="unread-count"]',
      '[data-tid*="unread"]',
      ".fui-Badge",
      '[aria-label*="unread" i] .fui-Badge',
    ]);
    const count = badge ? parseInt(badge.textContent || "0", 10) || 0 : 0;
    if (count === lastReportedCount) return;
    lastReportedCount = count;
    emit("teams-linux://set-unread-count", String(count)).catch((e) =>
      console.error("[teams-linux] emit unread count error", e)
    );
  }
  const observer = new MutationObserver(() => reportUnreadCount());
  observer.observe(document.body, { childList: true, subtree: true });
  reportUnreadCount();

  console.log("[teams-linux] мост GNOME интеграции загружен");
})();
