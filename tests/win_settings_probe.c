#define UNICODE
#include <windows.h>
#include <commctrl.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(int n,char**v){if(n<2)return 2;HWND s=FindWindowW(L"LumaSettings",NULL);HWND o=FindWindowW(L"LumaOverlay",NULL);
if(!strcmp(v[1],"settings")&&n>4){if(!s)return 3;SendMessageW(GetDlgItem(s,107),CB_SETCURSEL,atoi(v[2]),0);SendMessageW(GetDlgItem(s,100),CB_SETCURSEL,atoi(v[3]),0);SendMessageW(GetDlgItem(s,106),CB_SETCURSEL,atoi(v[4]),0);PostMessageW(s,WM_COMMAND,1,0);return 0;}
if(!strcmp(v[1],"color")&&n>4){HWND d=FindWindowW(L"#32770",NULL);if(!d)return 4;for(int i=0;i<3;i++){wchar_t b[12];swprintf(b,12,L"%d",atoi(v[i+2]));SetWindowTextW(GetDlgItem(d,706+i),b);}SendMessageW(d,WM_COMMAND,IDOK,0);return 0;}
if(!strcmp(v[1],"output")&&n>2){wchar_t p[1024];MultiByteToWideChar(CP_UTF8,0,v[2],-1,p,1024);if(!s)return 3;SetWindowTextW(GetDlgItem(s,101),p);PostMessageW(s,WM_COMMAND,1,0);return 0;}
if(!strcmp(v[1],"hover")&&n>3){SendMessageW(o,WM_MOUSEMOVE,0,MAKELPARAM(atoi(v[2]),atoi(v[3])));UpdateWindow(o);return 0;}
if(!strcmp(v[1],"selected")){HWND ex=FindWindowW(L"CabinetWClass",NULL);if(!ex)ex=FindWindowW(L"ExploreWClass",NULL);printf("explorer=%p\n",ex);return ex?0:5;}
if(!strcmp(v[1],"qrflag")){if(!s)return 3;SendMessageW(GetDlgItem(s,301),BM_SETCHECK,BST_CHECKED,0);PostMessageW(s,WM_COMMAND,1,0);return 0;}
return 2;}
