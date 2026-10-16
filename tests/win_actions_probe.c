#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(int n,char**v){if(n<2)return 2;HWND main=FindWindowW(L"LumaMain",NULL),text=FindWindowW(L"LumaText",NULL),overlay=FindWindowW(L"LumaOverlay",NULL);
if(!strcmp(v[1],"style")){HWND h=FindWindowW(L"LumaSettings",NULL);if(!h)return 5;SendMessageW(GetDlgItem(h,100),CB_SETCURSEL,1,0);SendMessageW(GetDlgItem(h,106),CB_SETCURSEL,1,0);SendMessageW(GetDlgItem(h,107),CB_SETCURSEL,0,0);PostMessageW(h,WM_COMMAND,1,0);return 0;}
if(!strcmp(v[1],"command")&&n>2){PostMessageW(main,WM_COMMAND,atoi(v[2]),0);return 0;}
if(!strcmp(v[1],"click")&&n>3){int x=atoi(v[2]),y=atoi(v[3]);SendMessageW(overlay,WM_LBUTTONDOWN,MK_LBUTTON,MAKELPARAM(x,y));SendMessageW(overlay,WM_LBUTTONUP,0,MAKELPARAM(x,y));return 0;}
if(!strcmp(v[1],"text")){if(!text)return 3;wchar_t b[8192];SendMessageW(GetDlgItem(text,100),WM_GETTEXT,8192,(LPARAM)b);char u[32768];WideCharToMultiByte(CP_UTF8,0,b,-1,u,32768,NULL,NULL);printf("text=%s\nopen_link_button=%d\n",u,GetDlgItem(text,3)!=NULL);return 0;}
if(!strcmp(v[1],"textcopy")){SendMessageW(text,WM_COMMAND,1,0);return 0;}
if(!strcmp(v[1],"textclose")){PostMessageW(text,WM_CLOSE,0,0);return 0;}
if(!strcmp(v[1],"cliptext")){if(!OpenClipboard(NULL))return 4;HANDLE h=GetClipboardData(CF_UNICODETEXT);if(!h){CloseClipboard();return 5;}wchar_t *p=GlobalLock(h);char u[32768];WideCharToMultiByte(CP_UTF8,0,p,-1,u,32768,NULL,NULL);printf("%s\n",u);GlobalUnlock(h);CloseClipboard();return 0;}
if(!strcmp(v[1],"notice")){PostMessageW(main,WM_APP+1,0,0x405);return 0;}
if(!strcmp(v[1],"pin")){HWND p=FindWindowW(L"LumaPin",NULL);printf("pinned=%d topmost=%d\n",p!=NULL,p?(GetWindowLongW(p,GWL_EXSTYLE)&WS_EX_TOPMOST)!=0:0);return p?0:5;}
if(!strcmp(v[1],"pinclose")){PostMessageW(FindWindowW(L"LumaPin",NULL),WM_CLOSE,0,0);return 0;}return 2;}
