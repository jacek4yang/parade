type LoginLocale = "en" | "zh-CN";

const translations: Record<string, string> = {
  "Read-only fleet observability": "只读服务器群可观测性",
  "Sign in to inspect accepted telemetry and administer Hub-side metadata.":
    "登录后可检查已接受的遥测，并管理 Hub 侧元数据。",
  "Administrator password": "管理员密码",
  "Sign in": "登录",
  "Monitored servers remain observation-only.": "受监测服务器始终仅允许观察。",
  "Checking credentials…": "正在验证凭据…",
  "Too many attempts. Try again later.": "尝试次数过多，请稍后再试。",
  "Access denied.": "访问被拒绝。",
  "The Hub is unreachable.": "无法连接 Hub。",
  Language: "语言",
  "sign in": "登录",
};

const form = document.querySelector<HTMLFormElement>("#login-form");
const password = document.querySelector<HTMLInputElement>("#password");
const message = document.querySelector<HTMLElement>("#login-message");
const language = document.querySelector<HTMLSelectElement>("#login-language");
const dashboardTitle =
  document.querySelector<HTMLElement>("#login-title")?.textContent || "Parade";

let locale: LoginLocale = (() => {
  const stored = localStorage.getItem("parade-locale");
  if (stored === "en" || stored === "zh-CN") return stored;
  return navigator.languages.some((item) => item.toLowerCase().startsWith("zh"))
    ? "zh-CN"
    : "en";
})();
let messageKey = "Monitored servers remain observation-only.";

function text(key: string): string {
  return locale === "zh-CN" ? (translations[key] ?? key) : key;
}

function applyLocale(): void {
  document.documentElement.lang = locale;
  document.title = `${dashboardTitle} · ${text("sign in")}`;
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((element) => {
    const key = element.dataset.i18n;
    if (key) element.textContent = text(key);
  });
  if (language) {
    language.value = locale;
    language.setAttribute("aria-label", text("Language"));
  }
  if (message) message.textContent = text(messageKey);
}

language?.addEventListener("change", () => {
  locale = language.value === "zh-CN" ? "zh-CN" : "en";
  localStorage.setItem("parade-locale", locale);
  applyLocale();
});

applyLocale();

form?.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (!password?.value || !message) return;
  messageKey = "Checking credentials…";
  message.textContent = text(messageKey);
  try {
    const response = await fetch("/api/v1/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password: password.value }),
    });
    if (response.ok) window.location.reload();
    else
      messageKey =
        response.status === 429
          ? "Too many attempts. Try again later."
          : "Access denied.";
    message.textContent = text(messageKey);
  } catch {
    messageKey = "The Hub is unreachable.";
    message.textContent = text(messageKey);
  } finally {
    password.value = "";
  }
});
