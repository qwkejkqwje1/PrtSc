#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
static HWND settings(){return FindWindowW(L"LumaSettings",NULL);}
int main(int n,char**v){if(n<2)return 2;HWND s=settings(),o=FindWindowW(L"LumaOverlay",NULL);
if(!strcmp(v[1],"configure")&&s){SetWindowTextW(GetDlgItem(s,101),L"Z:\\data\\qa06\\output");SetWindowTextW(GetDlgItem(s,114),L"Ctrl+Alt+S");SendMessageW(GetDlgItem(s,108),CB_SETCURSEL,1,0);SendMessageW(GetDlgItem(s,109),CB_SETCURSEL,0,0);SendMessageW(GetDlgItem(s,301),BM_SETCHECK,BST_CHECKED,0);SendMessageW(GetDlgItem(s,307),BM_SETCHECK,BST_CHECKED,0);SendMessageW(s,WM_COMMAND,1,0);return settings()?3:0;}
if(!strcmp(v[1],"trigger")){keybd_event(VK_CONTROL,0,0,0);keybd_event(VK_MENU,0,0,0);keybd_event('S',0,0,0);Sleep(50);keybd_event('S',0,KEYEVENTF_KEYUP,0);keybd_event(VK_MENU,0,KEYEVENTF_KEYUP,0);keybd_event(VK_CONTROL,0,KEYEVENTF_KEYUP,0);Sleep(300);return FindWindowW(L"LumaOverlay",NULL)?0:5;}
if(!strcmp(v[1],"conflict")&&s){if(!RegisterHotKey(NULL,87,MOD_CONTROL|MOD_ALT|MOD_NOREPEAT,'K'))return 8;SetWindowTextW(GetDlgItem(s,114),L"Ctrl+Alt+K");PostMessageW(s,WM_COMMAND,1,0);Sleep(600);printf("settings_still_open=%d error_dialog=%d\n",settings()!=NULL,FindWindowW(L"#32770",L"PrtSc")!=NULL);HWND d=FindWindowW(L"#32770",L"PrtSc");if(d)PostMessageW(d,WM_COMMAND,IDOK,0);UnregisterHotKey(NULL,87);return settings()?0:9;}
if(!strcmp(v[1],"cancelsettings")&&s){PostMessageW(s,WM_COMMAND,2,0);return 0;}
if(!strcmp(v[1],"cursoroff")&&s){SendMessageW(GetDlgItem(s,307),BM_SETCHECK,BST_UNCHECKED,0);SendMessageW(s,WM_COMMAND,1,0);return 0;}
if(!strcmp(v[1],"windows")){printf("overlay=%d text_dialog=%d settings=%d recording=%d\n",o!=NULL,FindWindowW(L"LumaText",NULL)!=NULL,s!=NULL,FindWindowW(L"LumaRecord",NULL)!=NULL);return 0;}
if(!strcmp(v[1],"closescene")){PostMessageW(FindWindowW(L"PrtScScene",NULL),WM_CLOSE,0,0);return 0;}
if(!strcmp(v[1],"cursorinfo")){CURSORINFO c={0};c.cbSize=sizeof(c);int ok=GetCursorInfo(&c);printf("cursor ok=%d flags=%lu handle=%p xy=%ld,%ld\n",ok,c.flags,c.hCursor,c.ptScreenPos.x,c.ptScreenPos.y);return 0;}
if(!strcmp(v[1],"clearclip")){if(!OpenClipboard(NULL))return 10;EmptyClipboard();CloseClipboard();return 0;}
if(!strcmp(v[1],"move")&&n==4){SetCursorPos(atoi(v[2]),atoi(v[3]));return 0;}
return 2;}
