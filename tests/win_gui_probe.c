#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static LPARAM xy(int x,int y){return MAKELPARAM(x,y);}
int main(int n,char**v){if(n<2)return 2;HWND h=FindWindowW(L"LumaOverlay",NULL);if(!strcmp(v[1],"exists")){printf("%d\n",h!=NULL);return h?0:3;}if(!strcmp(v[1],"settingssave")&&n==4){HWND s=FindWindowW(L"LumaSettings",NULL);if(!s)return 4;wchar_t path[512];MultiByteToWideChar(CP_UTF8,0,v[2],-1,path,512);SetWindowTextW(GetDlgItem(s,101),path);SendMessageW(GetDlgItem(s,102),CB_SETCURSEL,atoi(v[3]),0);SendMessageW(s,WM_COMMAND,MAKEWPARAM(102,CBN_SELCHANGE),(LPARAM)GetDlgItem(s,102));SetWindowTextW(GetDlgItem(s,103),L"63");SendMessageW(GetDlgItem(s,104),CB_SETCURSEL,2,0);SendMessageW(s,WM_COMMAND,1,0);return FindWindowW(L"LumaSettings",NULL)?5:0;}if(!h)return 3;
 if(!strcmp(v[1],"drag")&&n==6){int x=atoi(v[2]),y=atoi(v[3]),xx=atoi(v[4]),yy=atoi(v[5]);SendMessageW(h,WM_LBUTTONDOWN,MK_LBUTTON,xy(x,y));for(int i=1;i<=100;i++){SendMessageW(h,WM_MOUSEMOVE,MK_LBUTTON,xy(x+(xx-x)*i/100,y+(yy-y)*i/100));UpdateWindow(h);}SendMessageW(h,WM_LBUTTONUP,0,xy(xx,yy));UpdateWindow(h);return 0;}
 if(!strcmp(v[1],"double")){SendMessageW(h,WM_LBUTTONDBLCLK,MK_LBUTTON,xy(300,250));SendMessageW(h,WM_LBUTTONUP,0,xy(300,250));return FindWindowW(L"LumaOverlay",NULL)?0:4;}
 if(!strcmp(v[1],"cancel")){SendMessageW(h,WM_CANCELMODE,0,0);return FindWindowW(L"LumaOverlay",NULL)?0:4;}
 if(!strcmp(v[1],"paintbench")){LARGE_INTEGER a,b,f;QueryPerformanceFrequency(&f);SendMessageW(h,WM_KEYDOWN,'V',0);SendMessageW(h,WM_LBUTTONDOWN,MK_LBUTTON,xy(120,100));QueryPerformanceCounter(&a);for(int i=0;i<240;i++){SendMessageW(h,WM_MOUSEMOVE,MK_LBUTTON,xy(900+i%100,500+i%100));UpdateWindow(h);}QueryPerformanceCounter(&b);SendMessageW(h,WM_LBUTTONUP,0,xy(900,500));UpdateWindow(h);printf("240 forced selection paints: %.2f ms total, %.3f ms/paint (Wine, not native Windows)\n",(b.QuadPart-a.QuadPart)*1000./f.QuadPart,(b.QuadPart-a.QuadPart)*1000./f.QuadPart/240.);return 0;}
 if(!strcmp(v[1],"key")&&n==3){SendMessageW(h,WM_KEYDOWN,atoi(v[2]),0);UpdateWindow(h);return 0;}return 2;}
