#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static void utf8(wchar_t *s){int n=WideCharToMultiByte(CP_UTF8,0,s,-1,NULL,0,NULL,NULL);char *b=malloc(n);WideCharToMultiByte(CP_UTF8,0,s,-1,b,n,NULL,NULL);printf("%s",b);free(b);}
int main(int n,char**v){if(n<2)return 2;HWND t=FindWindowW(L"LumaTranslate",NULL),h=FindWindowW(L"LumaOverlay",NULL),m=FindWindowW(L"LumaMain",NULL);
if(!strcmp(v[1],"state")){printf("dialog=%d overlay=%d visible=%d direct=%d copy=%d\n",t!=NULL,h!=NULL,h?IsWindowVisible(h):0,t?IsWindowEnabled(GetDlgItem(t,1)):0,t?IsWindowEnabled(GetDlgItem(t,3)):0);return 0;}
if(!strcmp(v[1],"quit")){if(m)SendMessageW(m,WM_COMMAND,13,0);return 0;}
if(!strcmp(v[1],"command")&&n>2){if(!m)return 3;SendMessageW(m,WM_COMMAND,atoi(v[2]),0);return 0;}
if(!strcmp(v[1],"clipboard")){if(!OpenClipboard(NULL))return 5;HANDLE b=GetClipboardData(CF_UNICODETEXT);if(!b){CloseClipboard();return 6;}wchar_t *s=GlobalLock(b);utf8(s);GlobalUnlock(b);CloseClipboard();return 0;}
if(!t)return 3;
if(!strcmp(v[1],"set")&&n>2){FILE*f=fopen(v[2],"rb");if(!f)return 4;fseek(f,0,SEEK_END);long len=ftell(f);rewind(f);char*b=calloc(len+1,1);fread(b,1,len,f);fclose(f);int count=MultiByteToWideChar(CP_UTF8,0,b,-1,NULL,0);wchar_t*s=calloc(count,2);MultiByteToWideChar(CP_UTF8,0,b,-1,s,count);SetWindowTextW(GetDlgItem(t,100),s);SendMessageW(t,WM_COMMAND,MAKEWPARAM(100,EN_CHANGE),(LPARAM)GetDlgItem(t,100));free(b);free(s);return 0;}
if(!strcmp(v[1],"target")&&n>2){HWND c=GetDlgItem(t,101);SendMessageW(c,CB_SETCURSEL,atoi(v[2]),0);SendMessageW(t,WM_COMMAND,MAKEWPARAM(101,CBN_SELCHANGE),(LPARAM)c);return 0;}
if(!strcmp(v[1],"button")&&n>2){SendMessageW(t,WM_COMMAND,atoi(v[2]),0);return 0;}
if(!strcmp(v[1],"status")){wchar_t b[2048];GetWindowTextW(GetDlgItem(t,102),b,2048);utf8(b);puts("");return 0;}
return 2;}
