import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/DashboardView.vue'),
    },
    {
      path: '/search',
      name: 'search',
      component: () => import('@/views/HomeView.vue'),
    },
    {
      path: '/torrent/:hash',
      name: 'torrent',
      component: () => import('@/views/TorrentView.vue'),
      props: true,
    },
    {
      path: '/indexer',
      name: 'indexer',
      component: () => import('@/views/IndexerView.vue'),
    },
    {
      path: '/announcer',
      name: 'announcer',
      component: () => import('@/views/AnnouncerView.vue'),
    },
    {
      path: '/upload',
      name: 'upload',
      component: () => import('@/views/UploadView.vue'),
    },
    {
      path: '/system',
      name: 'system',
      component: () => import('@/views/SystemView.vue'),
      redirect: '/system/status',
      children: [
        {
          path: 'status',
          name: 'system-status',
          component: () => import('@/views/system/SystemStatus.vue'),
        },
        {
          path: 'general',
          name: 'system-general',
          component: () => import('@/views/system/SystemGeneral.vue'),
        },
        {
          path: 'sync',
          name: 'system-sync',
          component: () => import('@/views/system/SystemSync.vue'),
        },
        {
          path: 'api',
          name: 'system-api',
          component: () => import('@/views/system/SystemAPI.vue'),
        },
        {
          path: 'logs',
          name: 'system-logs',
          component: () => import('@/views/system/SystemLogs.vue'),
        },
      ],
    },
  ],
})

export default router
