#define UNICODE
#define _UNICODE
#include <windows.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
int main(int argc,char**argv){
 if(argc<2)return 2;
 HWND main=FindWindowW(L"LumaMain",NULL);
 if(!strcmp(argv[1],"tray")){if(!main)return 3;SetCursorPos(500,200);PostMessageW(main,WM_APP+1,0,WM_RBUTTONUP);return 0;}
 if(!strcmp(argv[1],"capture")){if(!main)return 3;PostMessageW(main,WM_APP+3,0,0);return 0;}
 if(!strcmp(argv[1],"quit")){if(!main)return 3;PostMessageW(main,WM_CLOSE,0,0);return 0;}
 if(!strcmp(argv[1],"settings")){HWND h=FindWindowW(L"LumaSettings",NULL);if(!h)return 3;if(argc>2&&!strcmp(argv[2],"history")){SendMessageW(GetDlgItem(h,300),BM_SETCHECK,BST_CHECKED,0);PostMessageW(h,WM_COMMAND,1,0);}return 0;}
 if(!strcmp(argv[1],"clipboard")&&argc>2){if(!OpenClipboard(NULL))return 4;HANDLE h=GetClipboardData(CF_DIB);if(!h){CloseClipboard();return 5;}const uint8_t*p=GlobalLock(h);if(!p)return 6;const BITMAPINFOHEADER*b=(const BITMAPINFOHEADER*)p;DWORD n=(DWORD)GlobalSize(h);BITMAPFILEHEADER f={0};f.bfType=0x4D42;f.bfSize=sizeof(f)+n;f.bfOffBits=sizeof(f)+b->biSize;FILE*out=fopen(argv[2],"wb");if(!out)return 7;fwrite(&f,sizeof(f),1,out);fwrite(p,n,1,out);fclose(out);printf("%ld x %ld, %u bytes\n",b->biWidth,b->biHeight,n);GlobalUnlock(h);CloseClipboard();return 0;}
 return 2;
}
