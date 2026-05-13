import { createRouter, createWebHashHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/launcher' },
    {
      path: '/launcher',
      name: 'launcher',
      component: () => import('@/views/LauncherView.vue'),
      meta: { title: '启动器', icon: 'i-carbon-grid' }
    },
    {
      path: '/resources',
      name: 'resources',
      component: () => import('@/views/ResourcesView.vue'),
      meta: { title: '资源归纳', icon: 'i-carbon-folder' }
    },
    {
      path: '/monitor',
      name: 'monitor',
      component: () => import('@/views/MonitorView.vue'),
      meta: { title: '性能监控', icon: 'i-carbon-chart-line' }
    },
    {
      path: '/ports',
      name: 'ports',
      component: () => import('@/views/PortsView.vue'),
      meta: { title: '端口监控', icon: 'i-carbon-network-1' }
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/SettingsView.vue'),
      meta: { title: '设置', icon: 'i-carbon-settings' }
    }
  ]
})
