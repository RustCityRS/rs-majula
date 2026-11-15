version=ps_2_0
ctab.flags=33024
ctab.constant=params,c0,1,2,type0
ctab.constant=pixelOffsetAndBloomScale,c1,1,6,type0
ctab.constant=sceneTex,s0,1,2,type1
ctab.string=params,"params"
ctab.padding=171,1
ctab.type=type0,vector,float,1,4,1
ctab.string=pixelOffsetAndBloomScale,"pixelOffsetAndBloomScale"
ctab.string=sceneTex,"sceneTex"
ctab.padding=171,2
ctab.type=type1,object,sampler2d,1,1,1
ctab.string=target,"ps_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c2.xyzw, 0.2126, 0.7152, 0.0722, 0
dcl position0, a0.xy
dcl sampler2d, s0.xyzw
add r0.xy, a0.xyzw, -c1.xyzw
texld r0.xyzw, r0.xyzw, s0.xyzw
dp3 r1.w, c2.xyzw, r0.xyzw
add r1.x, r1.wwww, -c0.xxxx
cmp r0.xyzw, r1.xxxx, r0.xyzw, c2.wwww
mov oC0.xyzw, r0.xyzw
