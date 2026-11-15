version=ps_2_0
ctab.flags=33024
ctab.constant=fogColour,c0,1,2,type0
ctab.string=fogColour,"fogColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, a0.xyz
dcl position0, v0.xyzw
mul r0.w, a0.zzzz, c0.wwww
lrp r1.xyzw, r0.wwww, c0.xyzw, v0.xyzw
max r0.x, r1.yyyy, r1.zzzz
max r2.w, r1.xxxx, r0.xxxx
rcp r0.x, r2.wwww
mul r0.xyzw, r0.xxxx, r1.xyzw
mov oC0.xyzw, r0.xyzw
