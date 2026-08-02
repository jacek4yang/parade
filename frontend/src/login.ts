const form = document.querySelector<HTMLFormElement>("#login-form");
const password = document.querySelector<HTMLInputElement>("#password");
const message = document.querySelector<HTMLElement>("#login-message");

form?.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (!password?.value || !message) return;
  message.textContent = "Checking credentials…";
  try {
    const response = await fetch("/api/v1/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password: password.value }),
    });
    if (response.ok) window.location.reload();
    else
      message.textContent =
        response.status === 429
          ? "Too many attempts. Try again later."
          : "Access denied.";
  } catch {
    message.textContent = "The Hub is unreachable.";
  } finally {
    password.value = "";
  }
});
