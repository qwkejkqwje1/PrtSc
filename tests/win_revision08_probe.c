#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <string.h>
int main(int n,char**v){HWND s=FindWindowW(L"LumaSettings",NULL);if(n<2)return 2;if(!strcmp(v[1],"snapshot-up")){keybd_event(VK_SNAPSHOT,0x37,KEYEVENTF_KEYUP,0);Sleep(250);return 0;}if(!strcmp(v[1],"focus-save")&&s){DWORD t=GetWindowThreadProcessId(s,NULL);AttachThreadInput(GetCurrentThreadId(),t,TRUE);SetForegroundWindow(s);SetFocus(GetDlgItem(s,1));AttachThreadInput(GetCurrentThreadId(),t,FALSE);return 0;}if(!strcmp(v[1],"snapshot-message-up")&&s){SendMessageW(GetDlgItem(s,114),WM_KEYUP,VK_SNAPSHOT,0);return 0;}return 2;}
