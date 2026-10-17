#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <shellapi.h>
int main(void){int n;LPWSTR *v=CommandLineToArgvW(GetCommandLineW(),&n);if(n>1){FILE*f=fopen("Z:\\data\\qa-translate\\opened-url.txt","ab");if(!f)return 2;int len=WideCharToMultiByte(CP_UTF8,0,v[1],-1,NULL,0,NULL,NULL);char*b=HeapAlloc(GetProcessHeap(),0,len);WideCharToMultiByte(CP_UTF8,0,v[1],-1,b,len,NULL,NULL);fwrite(b,1,len-1,f);fputc('\n',f);fclose(f);HeapFree(GetProcessHeap(),0,b);LocalFree(v);return 0;}LocalFree(v);
for(int i=0;i<2;i++){HKEY root=i?HKEY_CURRENT_USER:HKEY_CLASSES_ROOT,k;const wchar_t *base=i?L"Software\\Classes\\https":L"https",*command=i?L"Software\\Classes\\https\\shell\\open\\command":L"https\\shell\\open\\command";RegCreateKeyExW(root,base,0,NULL,0,KEY_WRITE,NULL,&k,NULL);RegSetValueExW(k,L"URL Protocol",0,REG_SZ,(const BYTE*)L"",2);RegCloseKey(k);RegCreateKeyExW(root,command,0,NULL,0,KEY_WRITE,NULL,&k,NULL);wchar_t cmd[]=L"\"Z:\\data\\qa-translate\\translate_sink.exe\" \"%1\"";RegSetValueExW(k,NULL,0,REG_SZ,(BYTE*)cmd,sizeof(cmd));RegCloseKey(k);}return 0;}
