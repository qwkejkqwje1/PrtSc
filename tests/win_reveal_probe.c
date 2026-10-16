#define UNICODE
#include <windows.h>
#include <shlobj.h>
#include <stdio.h>
int wmain(int n,wchar_t**v){if(n<2)return 2;CoInitializeEx(NULL,COINIT_APARTMENTTHREADED);PIDLIST_ABSOLUTE p=NULL;HRESULT a=SHParseDisplayName(v[1],NULL,&p,0,NULL);HRESULT b=E_FAIL;if(SUCCEEDED(a)){SHChangeNotify(SHCNE_CREATE,SHCNF_PATHW,v[1],NULL);b=SHOpenFolderAndSelectItems(p,0,NULL,0);CoTaskMemFree(p);}printf("parse_hresult=0x%08lX select_hresult=0x%08lX\n",a,b);CoUninitialize();return 0;}
