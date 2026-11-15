version=vs_2_0
ctab.flags=33024
ctab.constant=eyePosition,c19,1,78,type0
ctab.constant=fogParams,c15,1,62,type1
ctab.constant=fogPlanes,c14,1,58,type1
ctab.constant=modelMatrix,c2,3,10,type2
ctab.constant=modelViewMatrix,c6,4,26,type2
ctab.constant=projectionMatrix,c10,4,42,type2
ctab.constant=textureMatrix,c0,2,2,type3
ctab.constant=time,c17,1,70,type4
ctab.constant=waterParams,c18,1,74,type5
ctab.constant=waterPlane,c16,1,66,type1
ctab.string=eyePosition,"eyePosition"
ctab.type=type0,vector,float,1,3,1
ctab.string=fogParams,"fogParams"
ctab.padding=171,2
ctab.type=type1,vector,float,1,4,1
ctab.string=fogPlanes,"fogPlanes"
ctab.string=modelMatrix,"modelMatrix"
ctab.padding=171,2
ctab.type=type2,matrix_columns,float,4,4,1
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,1
ctab.type=type3,matrix_columns,float,4,2,1
ctab.string=time,"time"
ctab.padding=171,3
ctab.type=type4,scalar,float,1,1,1
ctab.string=waterParams,"waterParams"
ctab.type=type5,vector,float,1,2,1
ctab.string=waterPlane,"waterPlane"
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,1
def c5.xyzw, 0, 1, 0.0009765625, 0
dcl position0, v0.xyzw
dcl color0, v1.xyzw
dcl texcoord0, v2.xyzw
dcl texcoord1, v3.xyzw
rcp r0.x, -c19.yyyy
dp4 r1.y, v0.xyzw, c3.xyzw
dp4 r1.x, v0.xyzw, c2.xyzw
dp4 r1.z, v0.xyzw, c4.xyzw
add r0.y, r1.yyyy, -c19.yyyy
mul r0.x, r0.xxxx, r0.yyyy
lrp r2.xyz, r0.xxxx, r1.xyzw, c19.xyzw
add r0.xyz, r1.xyzw, -r2.xyzw
mov oT3.xyz, r1.xyzw
mul r1.xy, r1.xzzw, c18.yyyy
mul oT2.xy, r1.xyzw, c5.zzzz
dp3 r0.x, r0.xyzw, r0.xyzw
rsq r0.x, r0.xxxx
rcp r0.x, r0.xxxx
mul r0.x, r0.xxxx, c15.xxxx
max r0.x, r0.xxxx, c5.xxxx
min r0.x, r0.xxxx, c5.yyyy
mul oT1.x, r0.xxxx, c15.yyyy
dp4 r0.z, v0.xyzw, c8.xyzw
add r0.w, -r0.zzzz, -c14.zzzz
mov r1.y, c5.yyyy
mad r0.w, r0.wwww, -c14.wwww, r1.yyyy
max r0.w, r0.wwww, c5.xxxx
min r0.w, r0.wwww, c5.yyyy
mul r1.xyz, r0.wwww, c16.xyzw
dp4 r0.x, v0.xyzw, c6.xyzw
dp4 r0.y, v0.xyzw, c7.xyzw
mad r0.xyz, r1.xyzw, v3.xxxx, r0.xyzw
dp4 r0.w, v0.xyzw, c9.xyzw
dp4 oPos.x, r0.xyzw, c10.xyzw
dp4 oPos.y, r0.xyzw, c11.xyzw
dp4 oPos.z, r0.xyzw, c12.xyzw
dp4 oPos.w, r0.xyzw, c13.xyzw
mad r0.xyz, v2.xyxw, c5.yyxw, c5.xxyw
dp3 oT0.x, r0.xyzw, c0.xyww
dp3 oT0.y, r0.xyzw, c1.xyww
mov oD0.xyzw, v1.xyzw
mov oT2.z, c17.xxxx
mov oT2.w, v3.xxxx
