import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';
import * as {{lib-name}} from 'lib{{lib-name}}.so';

export default class EntryAbility extends UIAbility {
    onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
        console.info('EntryAbility onCreate');
        // Initialize Tauri/Rust
        console.info('lib{{lib-name}} keys:', Object.keys({{lib-name}}));
        try {
            if ({{lib-name}}.ohosInit) {
                {{lib-name}}.ohosInit();
                console.info('Tauri initialized');
            } else {
                console.error('ohosInit missing');
            }
        } catch (e) {
            console.error('Failed to initialize Tauri:', e);
        }
    }

    onWindowStageCreate(windowStage: window.WindowStage) {
        console.info('EntryAbility onWindowStageCreate');
        windowStage.loadContent('pages/Index', (err, data) => {
            if (err.code) {
                console.error('Failed to load the content. Cause: ' + JSON.stringify(err) ?? '');
                return;
            }
            console.info('Succeeded in loading the content. Data: ' + JSON.stringify(data) ?? '');
        });
    }
}
