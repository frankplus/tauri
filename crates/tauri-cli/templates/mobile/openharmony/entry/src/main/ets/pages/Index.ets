import { webview } from '@kit.ArkWeb';
import * as {{lib-name}} from 'lib{{lib-name}}.so';

@Entry
@Component
struct Index {
  controller: webview.WebviewController = new webview.WebviewController();

  aboutToAppear() {
    console.info('Index aboutToAppear');
    


    try {
      let injector = (script: string) => {
        console.info('Rust requested JS execution: ' + script);
        try {
          this.controller.runJavaScript(script);
        } catch (e) {
          console.error('runJavaScript failed: ' + e);
        }
      };

      // Register with Rust
      if ({{lib-name}}.registerWebview) {
        console.info('Calling {{lib-name}}.registerWebview...');
        {{lib-name}}.registerWebview(injector);
        console.info('{{lib-name}}.registerWebview called successfully');
      } else {
           console.error('registerWebview native export not found');
      }
    } catch (e) {
      console.error('Error during Webview registration: ' + e);
    }
  }

  build() {
    Stack() {
      // XComponent to drive the native event loop
      XComponent({
        id: 'tauri_xcomponent',
        type: 'surface',
        libraryname: '{{lib-name}}'
      })
      .onLoad((context) => {
        console.info('XComponent onLoad fired');
        console.info('{{lib-name}} keys: ' + JSON.stringify(Object.keys({{lib-name}})));
        if ({{lib-name}}.registerXcomponent) {
            {{lib-name}}.registerXcomponent(context);
        } else {
            console.error('registerXcomponent is not defined on generic {{lib-name}} object');
            // Fallback check for snake_case just in case, though unlikely with napi-rs defaults
            if ({{lib-name}}.register_xcomponent) {
                 console.warn('Found register_xcomponent (snake_case), using that instead.');
                 {{lib-name}}.register_xcomponent(context);
            }
        }
      })
      .onDestroy(() => {
        console.info('XComponent onDestroy');
      })
      .width('100%')
      .height('100%')
      .hitTestBehavior(HitTestMode.None) // Let touches pass through to Web? Or handled by Rust?

      Web({ src: "https://tauri.localhost/index.html", controller: this.controller })
        .domStorageAccess(true)
        .fileAccess(true)
        .imageAccess(true)
        .javaScriptAccess(true)
        .width('100%')
        .height('100%')
        .onInterceptRequest((event) => {
          if (event && event.request) {
            let url = event.request.getRequestUrl();
            if (url.startsWith("https://tauri.localhost")) {
              console.info("[Tauri] Intercepted: " + url);
              // Call Rust to resolve
              // Fix 'any' type error by explicit annotation and casting
              // Pass 'main' as window ID
              let content: string | null = {{lib-name}}.resolveRequest("main", url) as string | null;
              if (content) {
                console.info("[Tauri] Content resolved by Rust");
                
                let response = new WebResourceResponse();
                response.setResponseData(content);
                response.setResponseEncoding('utf-8');
                // Auto-detect mime type or default to text/plain for IPC?
                // For ipc:// it is usually JSON application/json, but let's see.
                let mimeType = url.endsWith(".html") ? 'text/html' : 
                               url.endsWith(".js") ? 'application/javascript' : 
                               url.endsWith(".css") ? 'text/css' : 
                               'text/plain';

                response.setResponseMimeType(mimeType);
                response.setResponseHeader([
                    { headerKey: "Access-Control-Allow-Origin", headerValue: "*" },
                    { headerKey: "Access-Control-Allow-Headers", headerValue: "*" },
                    { headerKey: "Access-Control-Allow-Methods", headerValue: "POST, GET, OPTIONS" }
                ]);
                response.setResponseCode(200);
                response.setReasonMessage('OK');
                response.setResponseIsReady(true);
                return response;
              }
            }
          }
          return null;
        })
        .onControllerAttached(() => {
          console.info('WebController Attached');
        })
        .javaScriptProxy({
            object: {
                postMessage: (msg: string) => {
                    console.info("IPC postMessage called with: " + msg);
                    if ({{lib-name}}.onIpcMessage) {
                        try {
                            console.info("Calling {{lib-name}}.onIpcMessage...");
                            {{lib-name}}.onIpcMessage("main", msg);
                            console.info("Called {{lib-name}}.onIpcMessage successfully");
                        } catch (e) {
                             console.error("Failed to call {{lib-name}}.onIpcMessage: " + e);
                        }
                    } else {
                        console.error("{{lib-name}}.onIpcMessage is not defined");
                    }
                }
            },
            name: "ipc",
            methodList: ["postMessage"],
            controller: this.controller,
        })
    }
    .width('100%')
    .height('100%')
  }
}
