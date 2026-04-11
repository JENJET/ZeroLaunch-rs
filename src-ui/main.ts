import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import router from './router/index'
import App from './App.vue'
import i18n from './i18n'
import { useRemoteConfigStore } from './stores/remote_config'

// 禁用右键菜单
document.addEventListener('contextmenu', (e) => {
    console.log("右键菜单被禁用")
    e.preventDefault()
})

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(ElementPlus)
app.use(router)
app.use(i18n)

// 初始化应用版本号（全局只调用一次）
const configStore = useRemoteConfigStore()
configStore.initAppVersion()

app.mount('#app')
