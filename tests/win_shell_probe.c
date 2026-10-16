#define UNICODE
#include <windows.h>
#include <commctrl.h>
#include <stdio.h>
#include <string.h>
static BOOL CALLBACK child(HWND h,LPARAM l){wchar_t c[128];GetClassNameW(h,c,128);if(!lstrcmpiW(c,L"SysListView32")){int n=(int)SendMessageW(h,LVM_GETITEMCOUNT,0,0),selected=(int)SendMessageW(h,LVM_GETNEXTITEM,(WPARAM)-1,LVNI_SELECTED);printf("list_count=%d selected_index=%d\n",n,selected);
DWORD pid;GetWindowThreadProcessId(h,&pid);HANDLE proc=OpenProcess(PROCESS_VM_OPERATION|PROCESS_VM_READ|PROCESS_VM_WRITE,FALSE,pid);
if(proc&&selected>=0){void*mem=VirtualAllocEx(proc,NULL,4096,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);LVITEMW item={0};item.iSubItem=0;item.cchTextMax=1000;item.pszText=(wchar_t*)((char*)mem+sizeof(item));WriteProcessMemory(proc,mem,&item,sizeof(item),NULL);SendMessageW(h,LVM_GETITEMTEXTW,selected,(LPARAM)mem);wchar_t text[1000]={0};ReadProcessMemory(proc,item.pszText,text,sizeof(text),NULL);char out[4000];WideCharToMultiByte(CP_UTF8,0,text,-1,out,sizeof(out),0,0);printf("selected_name=%s\n",out);VirtualFreeEx(proc,mem,0,MEM_RELEASE);}if(proc)CloseHandle(proc);}return TRUE;}
static BOOL CALLBACK closechild(HWND h,LPARAM l){wchar_t c[128];GetClassNameW(h,c,128);if(!lstrcmpiW(c,L"SysListView32"))PostMessageW(GetAncestor(h,GA_ROOT),WM_CLOSE,0,0);return TRUE;}
static BOOL CALLBACK top(HWND h,LPARAM l){EnumChildWindows(h,l?closechild:child,0);return TRUE;}
int main(int n,char**v){if(n<2)return 2;
 if(!strcmp(v[1],"pathcheck")){HWND s=FindWindowW(L"LumaSettings",NULL);wchar_t b[2048]={0};GetWindowTextW(GetDlgItem(s,101),b,2048);char out[8192];WideCharToMultiByte(CP_UTF8,0,b,-1,out,sizeof(out),0,0);printf("path=%s\nlen=%d\n",out,lstrlenW(b));printf("mkdir=%d error=%lu\n",CreateDirectoryW(L"Z:\\data\\qa05\\Тест, снимки",NULL),GetLastError());return 0;}
 if(!strcmp(v[1],"output")){HWND s=FindWindowW(L"LumaSettings",NULL);if(!s)return 3;SetWindowTextW(GetDlgItem(s,101),L"Z:\\data\\qa05\\Тест, снимки");PostMessageW(s,WM_COMMAND,1,0);return 0;}
 if(!strcmp(v[1],"dismiss")){HWND d=FindWindowW(L"#32770",NULL);if(d)PostMessageW(d,WM_COMMAND,IDOK,0);return 0;}
 if(!strcmp(v[1],"recordsettings")){HWND s=FindWindowW(L"LumaSettings",NULL);if(!s)return 3;SendMessageW(GetDlgItem(s,108),CB_SETCURSEL,1,0);SendMessageW(GetDlgItem(s,109),CB_SETCURSEL,0,0);SendMessageW(GetDlgItem(s,301),BM_SETCHECK,BST_CHECKED,0);PostMessageW(s,WM_COMMAND,1,0);return 0;}
 if(!strcmp(v[1],"closeexplorer")){EnumWindows(top,1);return 0;}
 if(!strcmp(v[1],"selected")){EnumWindows(top,0);return 0;}return 2;}
