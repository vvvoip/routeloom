import fs from 'node:fs/promises';
import path from 'node:path';
// Deterministic one-time project wiring; shared Swift is compiled into both Apple apps.
const root = path.resolve(import.meta.dirname, '../apps/client');
for (const os of ['ios','macos']) {
 const file=path.join(root,os,'Runner.xcodeproj/project.pbxproj');
 let text=(await fs.readFile(file,'utf8')).replaceAll('\r\n','\n');
 if(text.includes('A11000000000000000000001')) continue;
 const mac=os==='macos';
 const runner=mac?'33CC10EC2044A3C60003C045':'97C146ED1CF9000F007C117D';
 const main=mac?'33CC10E42044A3C60003C045':'97C146E51CF9000F007C117D';
 const product=mac?'33CC10EE2044A3C60003C045':'97C146EF1CF9000F007C117D';
 const sources=mac?'33CC10E92044A3C60003C045':'97C146EA1CF9000F007C117D';
 const project=mac?'33CC10E52044A3C60003C045':'97C146E61CF9000F007C117D';
 function editObject(id,fn) {
   const start=text.indexOf('\t\t'+id+' ');
   if(start<0)throw Error('Missing '+id);
   const end=text.indexOf('\n\t\t};',start)+6;
   text=text.slice(0,start)+fn(text.slice(start,end))+text.slice(end);
 }
 const ids={target:'A11000000000000000000001',product:'A11000000000000000000002',provider:'A11000000000000000000003',profile:'A11000000000000000000004',bridge:'A11000000000000000000005',sources:'A11000000000000000000006',frameworks:'A11000000000000000000007',resources:'A11000000000000000000008',configlist:'A11000000000000000000009',embed:'A11000000000000000000010',embedfile:'A11000000000000000000011',dependency:'A11000000000000000000012',proxy:'A11000000000000000000013',bp:'A11000000000000000000014',bpe:'A11000000000000000000015',bph:'A11000000000000000000016',bb:'A11000000000000000000017',group:'A11000000000000000000018'};
 let objects = '';
 const obj=(id,body)=>{objects+='\t\t'+id+' = {'+body+'};\n';};
 for(const [key,name] of [['provider','PacketTunnelProvider'],['profile','TunnelProfile'],['bridge','TunnelBridge']])
   obj(ids[key],'isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = "../apple/'+name+'.swift"; sourceTree = SOURCE_ROOT;');
 obj(ids.product,'isa = PBXFileReference; explicitFileType = "wrapper.app-extension"; path = PacketTunnel.appex; sourceTree = BUILT_PRODUCTS_DIR;');
 obj(ids.bp,'isa = PBXBuildFile; fileRef = '+ids.provider+';');
 obj(ids.bpe,'isa = PBXBuildFile; fileRef = '+ids.profile+';');
 obj(ids.bph,'isa = PBXBuildFile; fileRef = '+ids.profile+';');
 obj(ids.bb,'isa = PBXBuildFile; fileRef = '+ids.bridge+';');
 obj(ids.embedfile,'isa = PBXBuildFile; fileRef = '+ids.product+'; settings = {ATTRIBUTES = (RemoveHeadersOnCopy,);};');
 obj(ids.sources,'isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = ('+ids.bp+','+ids.bpe+',); runOnlyForDeploymentPostprocessing = 0;');
 obj(ids.frameworks,'isa = PBXFrameworksBuildPhase; buildActionMask = 2147483647; files = (); runOnlyForDeploymentPostprocessing = 0;');
 obj(ids.resources,'isa = PBXResourcesBuildPhase; buildActionMask = 2147483647; files = (); runOnlyForDeploymentPostprocessing = 0;');
 obj(ids.embed,'isa = PBXCopyFilesBuildPhase; buildActionMask = 2147483647; dstPath = ""; dstSubfolderSpec = 13; files = ('+ids.embedfile+',); name = "Embed App Extensions"; runOnlyForDeploymentPostprocessing = 0;');
 obj(ids.proxy,'isa = PBXContainerItemProxy; containerPortal = '+project+'; proxyType = 1; remoteGlobalIDString = '+ids.target+'; remoteInfo = PacketTunnel;');
 obj(ids.dependency,'isa = PBXTargetDependency; target = '+ids.target+'; targetProxy = '+ids.proxy+';');
 obj(ids.group,'isa = PBXGroup; children = ('+[ids.provider,ids.profile,ids.bridge].join(',')+',); name = SharedTunnel; sourceTree = "<group>";');
 obj(ids.target,'isa = PBXNativeTarget; buildConfigurationList = '+ids.configlist+'; buildPhases = ('+[ids.sources,ids.frameworks,ids.resources].join(',')+',); buildRules = (); dependencies = (); name = PacketTunnel; productName = PacketTunnel; productReference = '+ids.product+'; productType = "com.apple.product-type.app-extension";');
 const configs=[];
 for(const [i,name] of ['Debug','Release','Profile'].entries()) {
   const id='A1100000000000000000002'+i; configs.push(id);
   obj(id,'isa = XCBuildConfiguration; buildSettings = {APPLICATION_EXTENSION_API_ONLY = YES; CLANG_ENABLE_MODULES = YES; CODE_SIGN_STYLE = Automatic; CODE_SIGN_ENTITLEMENTS = PacketTunnel/PacketTunnel.entitlements; CURRENT_PROJECT_VERSION = 1; GENERATE_INFOPLIST_FILE = NO; INFOPLIST_FILE = PacketTunnel/Info.plist; LD_RUNPATH_SEARCH_PATHS = ("$(inherited)", "@executable_path/Frameworks", "@executable_path/../../Frameworks",); MARKETING_VERSION = 0.1.0; PRODUCT_BUNDLE_IDENTIFIER = org.routeloom.routeloom.PacketTunnel; PRODUCT_NAME = "$(TARGET_NAME)"; SKIP_INSTALL = YES; SWIFT_VERSION = 5.0; SWIFT_OPTIMIZATION_LEVEL = '+(name==='Debug'?'"-Onone"':'"-O"')+'; '+(mac?'SDKROOT = macosx; MACOSX_DEPLOYMENT_TARGET = 12.0; ENABLE_APP_SANDBOX = YES;':'SDKROOT = iphoneos; IPHONEOS_DEPLOYMENT_TARGET = 15.0; TARGETED_DEVICE_FAMILY = "1,2";')+' }; name = '+name+';');
 }
 obj(ids.configlist,'isa = XCConfigurationList; buildConfigurations = ('+configs.join(',')+',); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;');
 editObject(runner,body=>body.replace('buildPhases = (','buildPhases = (\n\t\t\t\t'+ids.embed+',').replace('dependencies = (','dependencies = (\n\t\t\t\t'+ids.dependency+','));
 editObject(project,body=>body.replace('targets = (','targets = (\n\t\t\t\t'+ids.target+','));
 editObject(main,body=>body.replace('children = (','children = (\n\t\t\t\t'+ids.group+','));
 editObject(product,body=>body.replace('children = (','children = (\n\t\t\t\t'+ids.product+','));
 editObject(sources,body=>body.replace('files = (','files = (\n\t\t\t\t'+ids.bph+',\n\t\t\t\t'+ids.bb+','));
 if(!mac)text=text.replaceAll('PRODUCT_BUNDLE_IDENTIFIER = org.routeloom.routeloom;','CODE_SIGN_ENTITLEMENTS = Runner/Runner.entitlements;\n\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = org.routeloom.routeloom;');
 text=text.replace('\n\tobjects = {','\n\tobjects = {\n'+objects);
 await fs.writeFile(file,text);
 console.log('Wired '+os+' packet tunnel target');
}
