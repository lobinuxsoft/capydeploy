<template>
  <NuxtLink to="/" class="logo-link">
    <img src="/logo.jpg" alt="CapyDeploy" class="logo-img" />
    <span class="logo-text">CapyDeploy</span>
  </NuxtLink>
</template>

<style scoped>
.logo-link {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  text-decoration: none;
  transition: transform 0.2s ease;
}

.logo-link:hover {
  transform: scale(1.05);
}

.logo-img {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  object-fit: cover;
  box-shadow: 0 2px 8px rgba(249, 115, 22, 0.3);
}

.logo-text {
  font-size: 1.25rem;
  font-weight: 700;
  background: linear-gradient(135deg, #f97316 0%, #fb923c 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

@media (max-width: 640px) {
  .logo-text {
    display: none;
  }
}
</style>
