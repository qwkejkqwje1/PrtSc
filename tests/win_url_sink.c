#define UNICODE
#include <windows.h>
#include <stdio.h>
int main(int argc,char**argv){if(argc>1){FILE*f=fopen("Z:\\data\\qa08\\opened-url.txt","a");fprintf(f,"%s\n",argv[1]);fclose(f);return 0;}HKEY k;RegCreateKeyExW(HKEY_CURRENT_USER,L"Software\\Classes\\http",0,NULL,0,KEY_WRITE,NULL,&k,NULL);RegSetValueExW(k,L"URL Protocol",0,REG_SZ,(const BYTE*)L"",2);RegCloseKey(k);RegCreateKeyExW(HKEY_CURRENT_USER,L"Software\\Classes\\http\\shell\\open\\command",0,NULL,0,KEY_WRITE,NULL,&k,NULL);wchar_t cmd[]=L"\"Z:\\data\\qa08\\url_sink.exe\" \"%1\"";RegSetValueExW(k,NULL,0,REG_SZ,(BYTE*)cmd,sizeof(cmd));RegCloseKey(k);return 0;}
