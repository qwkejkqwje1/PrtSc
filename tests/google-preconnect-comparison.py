import os,subprocess,socket,time,json,select
base='/data/prtsc07/PrtSc-0.6.0'
runs=[]
for label,exe in [('0.6',base+'/PrtSc.exe'),('0.7',base+'/target/x86_64-pc-windows-gnu/release/PrtSc.exe')]:
 for idle_n in [0,1,8]:
  p=subprocess.Popen(['wine64',exe,'--google-test-server','--fast-search'],stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
  if not select.select([p.stdout],[],[],15)[0]: p.kill();raise RuntimeError('No URL')
  url=p.stdout.readline().decode().strip();host,path=url.removeprefix('http://').split('/',1);h,port=host.split(':');addr=(h,int(port));idle=[socket.create_connection(addr) for _ in range(idle_n)];time.sleep(.03)
  start=time.perf_counter();s=socket.create_connection(addr);s.sendall(f'GET /{path} HTTP/1.1\r\nHost: {host}\r\n\r\n'.encode());s.settimeout(12);buf=b''
  while b:=s.recv(65536):buf+=b
  ms=(time.perf_counter()-start)*1000
  assert buf.startswith(b'HTTP/1.1 200'),buf[:100]
  runs.append(dict(version=label,idleConnections=idle_n,localResponseMs=round(ms,2),responseBytes=len(buf)))
  for i in idle:i.close()
  s.close();p.wait(timeout=5)
json.dump(dict(note='Same Windows binaries under Wine; synthetic 888x436 fast JPEG; local response only, NOT browser startup or Google result latency',runs=runs),open('/data/qa07/google-preconnect-comparison.json','w'),indent=2)
print(json.dumps(runs))
