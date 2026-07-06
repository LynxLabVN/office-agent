import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'

const term = new Terminal({
  cursorBlink: true,
  fontSize: 14,
  fontFamily: 'ui-monospace, "SF Mono", Menlo, Consolas, monospace',
  theme: {
    background: '#0b1220',
    foreground: '#e2e8f0',
    cursor: '#94a3b8',
    selectionBackground: '#243049'
  }
})

const fitAddon = new FitAddon()
term.loadAddon(fitAddon)

const terminalEl = document.getElementById('terminal')!
term.open(terminalEl)
fitAddon.fit()

let sessionId: number | null = null
let dataListener: (() => void) | null = null
let exitListener: (() => void) | null = null

const statusEl = document.getElementById('status') as HTMLSpanElement
const startBtn = document.getElementById('start') as HTMLButtonElement
const closeBtn = document.getElementById('close') as HTMLButtonElement

function setStatus(msg: string) {
  statusEl.textContent = msg
}

async function startSession() {
  if (sessionId !== null) return

  setStatus('starting…')
  try {
    const { id } = await invoke<{ id: number }>('terminal_start', {
      options: {
        shell: null,
        cwd: null,
        rows: fitAddon.proposeDimensions()?.rows ?? 24,
        cols: fitAddon.proposeDimensions()?.cols ?? 80
      }
    })
    sessionId = id
    setStatus(`session ${id} running`)
    startBtn.disabled = true
    closeBtn.disabled = false

    dataListener = await listen(`terminal:${id}:data`, (event) => {
      const data = event.payload as number[]
      const str = new Uint8Array(data).reduce((acc, b) => acc + String.fromCharCode(b), '')
      term.write(str)
    })

    exitListener = await listen(`terminal:${id}:exit`, (event) => {
      setStatus(`session ${id} exited (${JSON.stringify(event.payload)})`)
      cleanupSession()
    })
  } catch (e) {
    setStatus(`start failed: ${e}`)
  }
}

async function writeToSession(data: string) {
  if (sessionId === null) return
  try {
    await invoke('terminal_write', { id: sessionId, data })
  } catch (e) {
    setStatus(`write failed: ${e}`)
  }
}

async function resizeSession(rows: number, cols: number) {
  if (sessionId === null) return
  try {
    await invoke('terminal_resize', { id: sessionId, rows, cols })
  } catch (e) {
    setStatus(`resize failed: ${e}`)
  }
}

async function closeSession() {
  if (sessionId === null) return
  try {
    await invoke('terminal_dispose', { id: sessionId })
  } catch (e) {
    setStatus(`close failed: ${e}`)
  }
  cleanupSession()
}

function cleanupSession() {
  dataListener?.()
  exitListener?.()
  dataListener = null
  exitListener = null
  sessionId = null
  startBtn.disabled = false
  closeBtn.disabled = true
  setStatus('no session')
}

startBtn.addEventListener('click', startSession)
closeBtn.addEventListener('click', closeSession)

term.onData((data) => {
  void writeToSession(data)
})

const resizeObserver = new ResizeObserver(() => {
  fitAddon.fit()
  const dims = fitAddon.proposeDimensions()
  if (dims) {
    void resizeSession(dims.rows, dims.cols)
  }
})
resizeObserver.observe(terminalEl)

setStatus('no session')
