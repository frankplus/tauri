import { webview } from '@kit.ArkWeb';
import { util } from '@kit.ArkTS';
import { common } from '@kit.AbilityKit';
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
              
              let response = new WebResourceResponse();
              
              // Remove query parameters for Mime type detection and RawFile path
              let cleanUrl = url.split('?')[0].split('#')[0];
              let path = cleanUrl.replace("https://tauri.localhost/", "");
              if (path === "") { path = "index.html"; }
              if (path.endsWith("/")) { path += "index.html"; }

              // MimeType detection
              let mimeType = 'text/plain';
                if (cleanUrl.endsWith(".html")) { mimeType = 'text/html'; }
                else if (cleanUrl.endsWith(".js") || cleanUrl.endsWith(".mjs")) { mimeType = 'application/javascript'; }
                else if (cleanUrl.endsWith(".css")) { mimeType = 'text/css'; }
                else if (cleanUrl.endsWith(".png")) { mimeType = 'image/png'; }
                else if (cleanUrl.endsWith(".jpg") || cleanUrl.endsWith(".jpeg")) { mimeType = 'image/jpeg'; }
                else if (cleanUrl.endsWith(".svg")) { mimeType = 'image/svg+xml'; }
                else if (cleanUrl.endsWith(".json")) { mimeType = 'application/json'; }
                else if (cleanUrl.endsWith(".ico")) { mimeType = 'image/x-icon'; }
                
              response.setResponseMimeType(mimeType);
              response.setResponseHeader([
                    { headerKey: "Access-Control-Allow-Origin", headerValue: "*" },
                    { headerKey: "Access-Control-Allow-Headers", headerValue: "*" },
                    { headerKey: "Access-Control-Allow-Methods", headerValue: "POST, GET, OPTIONS" }
              ]);
              response.setResponseCode(200);
              response.setReasonMessage('OK');
              response.setResponseIsReady(true);

              // Strategy 1: Try RawFile (Direct Asset Serving)
              try {
                  let context = getContext(this) as common.UIAbilityContext;
                  let content = context.resourceManager.getRawFileContentSync(path);
                  console.info("[Tauri] Served from RawFile: " + path + " (Size: " + content.length + ")");
                  
                  // content is Uint8Array
                  
                  // Special injection for index.html
                  if (path === "index.html") {
                       let textDecoder = util.TextDecoder.create("utf-8");
                       let htmlStr = textDecoder.decodeWithStream(content);
                       
                       // Fetch init scripts from Rust
                       let initContent: Uint8Array | null = {{lib-name}}.resolveRequest("main", "https://tauri.localhost/__tauri_init__") as Uint8Array | null;
                       if (initContent) {
                            // Convert to string
                            let initData: Uint8Array = (initContent instanceof Uint8Array) ? initContent : new Uint8Array(Object.values(initContent) as number[]);
                            let initStr = textDecoder.decodeWithStream(initData);
                            
                            // Inject before </head>
                            let injectedHtml = htmlStr.replace("</head>", initStr + "</head>");
                            response.setResponseData(injectedHtml);
                            response.setResponseEncoding('utf-8');
                            return response;
                       }
                  }

                  // Standard serving
                  let isText = mimeType.startsWith("text/") || 
                             mimeType === "application/javascript" || 
                             mimeType === "application/json" ||
                             mimeType === "image/svg+xml";
                  
                   if (isText) {
                        try {
                            let textDecoder = util.TextDecoder.create("utf-8");
                            let str = textDecoder.decodeWithStream(content);
                            response.setResponseData(str);
                            response.setResponseEncoding('utf-8');
                        } catch (e) {
                             response.setResponseData(content.buffer.slice(content.byteOffset, content.byteOffset + content.byteLength));
                             response.setResponseEncoding('utf-8');
                        }
                   } else {
                        response.setResponseData(content.buffer.slice(content.byteOffset, content.byteOffset + content.byteLength));
                   }
                   return response;

              } catch (e) {
                  // Only warn if it's not a common expected failure
                  // console.warn("[Tauri] RawFile not found or failed: " + path);
              }

              // Strategy 2: Call Rust (Plugins/Virtual Assets/IPC)
              // Fix 'any' type error by explicit annotation and casting
              // Pass 'main' as window ID
              let content: Uint8Array | null = {{lib-name}}.resolveRequest("main", url) as Uint8Array | null;
              if (content) {
                // Fix: content might be a JS Array<number> or object from NAPI
                let data: Uint8Array;
                if (content instanceof Uint8Array) {
                    data = content;
                } else if (Array.isArray(content) || (content && typeof content === 'object')) {
                     try {
                        // This handles generic object or array-like structure returned by NAPI
                        // explicit casting for strict mode
                        data = new Uint8Array(Object.values(content) as number[]);
                     } catch (e) { 
                         console.error("[Tauri] Failed to convert content to Uint8Array: " + e);
                         return null; 
                     }
                } else {
                    console.error("[Tauri] Unknown content type returned from Rust");
                    return null;
                }
                
                console.info("[Tauri] Content resolved by Rust. Size: " + data.length);
                
                let isText = mimeType.startsWith("text/") || 
                             mimeType === "application/javascript" || 
                             mimeType === "application/json" ||
                             mimeType === "image/svg+xml";

                if (isText) {
                    try {
                        let textDecoder = util.TextDecoder.create("utf-8");
                        let str = textDecoder.decodeWithStream(data);
                        response.setResponseData(str);
                        response.setResponseEncoding('utf-8');
                    } catch (e) {
                         response.setResponseData(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
                         response.setResponseEncoding('utf-8');
                    }
                } else {
                    if (data.buffer) {
                         let buffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
                         response.setResponseData(buffer);
                    } else {
                         // Fallback for safety
                         response.setResponseData(data);
                    }
                }

                return response;
              } else {
                 console.warn("[Tauri] 404 Not Found: " + url);
                 return null;
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
