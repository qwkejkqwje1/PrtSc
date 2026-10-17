#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static LPARAM xy(int x,int y){return MAKELPARAM(x,y);}
int main(int n,char **v){if(n<2)return 2;HWND h=FindWindowW(L"LumaOverlay",NULL),m=FindWindowW(L"LumaMain",NULL),s=FindWindowW(L"LumaSettings",NULL);
if(!strcmp(v[1],"state")){printf("overlay=%d settings=%d main=%d\n",h!=NULL,s!=NULL,m!=NULL);return 0;}
if(!strcmp(v[1],"command")&&n>2){if(!m)return 3;SendMessageW(m,WM_COMMAND,atoi(v[2]),0);return 0;}
if(!strcmp(v[1],"resources")){DWORD pid=0;GetWindowThreadProcessId(m,&pid);HANDLE p=OpenProcess(PROCESS_QUERY_INFORMATION|PROCESS_VM_READ,FALSE,pid);printf("gdi=%lu user=%lu\n",GetGuiResources(p,0),GetGuiResources(p,1));CloseHandle(p);return 0;}
if(!strcmp(v[1],"icons")&&n>3){wchar_t path[1024];MultiByteToWideChar(CP_UTF8,0,v[2],-1,path,1024);HMODULE lib=LoadLibraryExW(path,NULL,LOAD_LIBRARY_AS_DATAFILE);HICON a=LoadImageW(lib,MAKEINTRESOURCEW(1),IMAGE_ICON,32,32,0),b=LoadImageW(lib,MAKEINTRESOURCEW(1),IMAGE_ICON,16,16,0);if(!a||!b)return 7;HDC screen=GetDC(NULL),dc=CreateCompatibleDC(screen);BITMAPINFO bi={0};bi.bmiHeader.biSize=40;bi.bmiHeader.biWidth=96;bi.bmiHeader.biHeight=-48;bi.bmiHeader.biPlanes=1;bi.bmiHeader.biBitCount=32;void *bits;HBITMAP bm=CreateDIBSection(screen,&bi,DIB_RGB_COLORS,&bits,NULL,0);HGDIOBJ old=SelectObject(dc,bm);memset(bits,0x40,96*48*4);DrawIconEx(dc,8,8,a,32,32,0,NULL,DI_NORMAL);DrawIconEx(dc,64,16,b,16,16,0,NULL,DI_NORMAL);GdiFlush();BITMAPFILEHEADER f={0};f.bfType=0x4d42;f.bfOffBits=sizeof(f)+40;f.bfSize=f.bfOffBits+96*48*4;FILE*out=fopen(v[3],"wb");fwrite(&f,sizeof(f),1,out);fwrite(&bi.bmiHeader,40,1,out);fwrite(bits,96*48*4,1,out);fclose(out);SelectObject(dc,old);DeleteObject(bm);DeleteDC(dc);ReleaseDC(NULL,screen);DestroyIcon(a);DestroyIcon(b);FreeLibrary(lib);puts("embedded icons 32/16 OK");return 0;}
if(!h)return 3;
if(!strcmp(v[1],"refresh")){InvalidateRect(h,NULL,FALSE);UpdateWindow(h);return 0;}
if(!strcmp(v[1],"hold")&&n==6){int x=atoi(v[2]),y=atoi(v[3]),xx=atoi(v[4]),yy=atoi(v[5]);SendMessageW(h,WM_LBUTTONDOWN,MK_LBUTTON,xy(x,y));for(int i=1;i<=30;i++){SendMessageW(h,WM_MOUSEMOVE,MK_LBUTTON,xy(x+(xx-x)*i/30,y+(yy-y)*i/30));UpdateWindow(h);}return 0;}
if(!strcmp(v[1],"up")){SendMessageW(h,WM_LBUTTONUP,0,0);UpdateWindow(h);return 0;}
if((!strcmp(v[1],"text")||!strcmp(v[1],"text-file"))&&n>2){wchar_t text[4096];char bytes[8192];const char *input=v[2];if(!strcmp(v[1],"text-file")){FILE*f=fopen(v[2],"rb");if(!f)return 9;size_t len=fread(bytes,1,sizeof(bytes)-1,f);bytes[len]=0;fclose(f);input=bytes;}MultiByteToWideChar(CP_UTF8,0,input,-1,text,4096);for(wchar_t *p=text;*p;p++){if(*p=='\n'){keybd_event(VK_SHIFT,0,0,0);Sleep(70);SendMessageW(h,WM_CHAR,13,0);keybd_event(VK_SHIFT,0,KEYEVENTF_KEYUP,0);Sleep(70);}else SendMessageW(h,WM_CHAR,*p,0);}UpdateWindow(h);return 0;}
if(!strcmp(v[1],"stroke-bench")){LARGE_INTEGER a,b,f;QueryPerformanceFrequency(&f);SendMessageW(h,WM_KEYDOWN,'P',0);QueryPerformanceCounter(&a);for(int k=0;k<80;k++){int x=220+(k%10)*65,y=300+(k/10)*25;SendMessageW(h,WM_LBUTTONDOWN,MK_LBUTTON,xy(x,y));for(int j=1;j<=8;j++){SendMessageW(h,WM_MOUSEMOVE,MK_LBUTTON,xy(x+j*3,y+j));UpdateWindow(h);}SendMessageW(h,WM_LBUTTONUP,0,xy(x+24,y+8));UpdateWindow(h);}QueryPerformanceCounter(&b);printf("80 strokes / 640 forced moves: %.2f ms (Wine only)\n",(b.QuadPart-a.QuadPart)*1000./f.QuadPart);return 0;}
return 2;}
