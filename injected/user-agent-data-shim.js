// user-agent-data-shim.js
//
// WebKit НЕ реализует navigator.userAgentData (User-Agent Client Hints) —
// это чисто Chromium-фича (Chrome, Edge, Opera и другие Chromium-браузеры).
// Firefox и Safari её не поддерживают вовсе:
// https://developer.mozilla.org/en-US/docs/Web/API/User-Agent_Client_Hints_API
//
// Наша строка User-Agent (см. src/main.rs) выдаёт webkit2gtk за
// Chrome/126, но саму строку легко подделать, и сайты вроде Teams всё чаще
// сверяются с navigator.userAgentData как с более надёжным сигналом. Без
// этого шима получается явное противоречие: UA-строка говорит "я Chrome",
// а navigator.userAgentData === undefined — чего у настоящего Chrome не
// бывает никогда. Это может быть причиной, по которой кнопки звонка
// остаются disabled даже после подмены UA-строки: Teams видит несовпадение
// сигналов и просто не доверяет "браузеру".
//
// ВАЖНО: этот файл подключается через Tauri initialization_script
// (document-start, см. src/main.rs), а НЕ через on_page_load — он обязан
// выполниться до того, как заработает любой скрипт самого Teams, иначе
// проверка совместимости браузера уже пройдёт и не заметит объект.
(() => {
  if (window.navigator.userAgentData) return; // уже есть — ничего не трогаем

  const brands = [
    { brand: "Not)A;Brand", version: "24" },
    { brand: "Chromium", version: "126" },
    { brand: "Google Chrome", version: "126" },
  ];

  const uaData = {
    brands,
    mobile: false,
    platform: "Linux",
    toJSON() {
      return { brands, mobile: false, platform: "Linux" };
    },
    getHighEntropyValues(hints) {
      const full = {
        architecture: "x86",
        bitness: "64",
        model: "",
        platformVersion: "6.8.0",
        uaFullVersion: "126.0.6478.126",
        fullVersionList: brands,
        wow64: false,
      };
      const result = { brands, mobile: false, platform: "Linux" };
      (hints || []).forEach((hint) => {
        if (hint in full) result[hint] = full[hint];
      });
      return Promise.resolve(result);
    },
  };

  try {
    Object.defineProperty(window.navigator, "userAgentData", {
      value: uaData,
      configurable: true,
      enumerable: true,
    });
  } catch (e) {
    console.warn("[teams-linux] Не удалось подставить navigator.userAgentData:", e);
  }
})();
