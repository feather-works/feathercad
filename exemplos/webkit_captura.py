#!/usr/bin/env python3
"""Abre uma página no WebKitGTK (o MESMO motor do app Tauri no Linux), fora da tela, lê o texto do elemento `#log`
e grava uma captura PNG. Serve para conferir renderização (canvas, Worker, OffscreenCanvas) sem ver a janela do app.

    WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 \
        python3 exemplos/webkit_captura.py http://127.0.0.1:8765/index.html captura.png

(O `OffscreenWindow` não oferece OpenGL: por isso as duas variáveis, que só valem para este teste. Página que pinta de
forma assíncrona deve segurar o evento `load` com algo lento, ou esperar mais; aqui a leitura é 1,5 s depois do load.)
"""
import gi, sys
gi.require_version('Gtk','3.0'); gi.require_version('WebKit2','4.1')
from gi.repository import Gtk, WebKit2, GLib

url, saida = sys.argv[1], sys.argv[2]
janela = Gtk.OffscreenWindow(); janela.set_default_size(1010, 700)
wv = WebKit2.WebView()
janela.add(wv); janela.show_all()
ajustes = wv.get_settings()
print("política de aceleração:", ajustes.get_hardware_acceleration_policy(), "| WebGL:", ajustes.get_enable_webgl())
def console(_wv, msg, *a): pass
def depois():
    wv.run_javascript("document.getElementById('log').textContent + ' | UA=' + navigator.userAgent.slice(-40) + ' | offscreen=' + (typeof OffscreenCanvas) + ' worker=' + (typeof Worker)", None, resultado)
    return False
def resultado(_wv, res, *_):
    try:
        v = wv.run_javascript_finish(res).get_js_value().to_string()
        print("página diz:", v)
    except Exception as e:
        print("erro js:", e)
    def foto(_w, r, *_):
        try:
            surf = wv.get_snapshot_finish(r); surf.write_to_png(saida); print("captura gravada:", saida)
        except Exception as e:
            print("erro captura:", e)
        Gtk.main_quit()
    wv.get_snapshot(WebKit2.SnapshotRegion.VISIBLE, WebKit2.SnapshotOptions.NONE, None, foto)
wv.connect("load-changed", lambda w, ev: GLib.timeout_add(1500, depois) if ev == WebKit2.LoadEvent.FINISHED else None)
GLib.timeout_add(25000, lambda: (print("tempo esgotado"), Gtk.main_quit()))
wv.load_uri(url)
Gtk.main()
