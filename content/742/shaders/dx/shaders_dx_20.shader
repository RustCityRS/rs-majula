version=vs_2_0
ctab.flags=33024
ctab.constant=ambientColour,c1,1,6,type0
ctab.constant=eyePosition,c22,1,90,type0
ctab.constant=fogParams,c18,1,74,type1
ctab.constant=fogPlanes,c17,1,70,type1
ctab.constant=modelMatrix,c5,3,22,type2
ctab.constant=modelViewMatrix,c9,4,38,type2
ctab.constant=projectionMatrix,c13,4,54,type2
ctab.constant=sunColour,c0,1,2,type0
ctab.constant=sunDirection,c2,1,10,type0
ctab.constant=textureMatrix,c3,2,14,type3
ctab.constant=time,c20,1,82,type4
ctab.constant=waterParams,c21,1,86,type5
ctab.constant=waterPlane,c19,1,78,type1
ctab.string=ambientColour,"ambientColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=eyePosition,"eyePosition"
ctab.string=fogParams,"fogParams"
ctab.padding=171,2
ctab.type=type1,vector,float,1,4,1
ctab.string=fogPlanes,"fogPlanes"
ctab.string=modelMatrix,"modelMatrix"
ctab.padding=171,2
ctab.type=type2,matrix_columns,float,4,4,1
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=sunColour,"sunColour"
ctab.string=sunDirection,"sunDirection"
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,2
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
def c8.xyzw, 0, 1, 0.5, 0.0009765625
dcl position0, v0.xyzw
dcl normal0, v1.xyzw
dcl color0, v2.xyzw
dcl texcoord0, v3.xyzw
dcl texcoord1, v4.xyzw
rcp r0.x, -c22.yyyy
dp4 r1.y, v0.xyzw, c6.xyzw
dp4 r1.x, v0.xyzw, c5.xyzw
dp4 r1.z, v0.xyzw, c7.xyzw
add r0.y, r1.yyyy, -c22.yyyy
mul r0.x, r0.xxxx, r0.yyyy
lrp r2.xyz, r0.xxxx, r1.xyzw, c22.xyzw
add r0.xyz, r1.xyzw, -r2.xyzw
mov oT3.xyz, r1.xyzw
mul r1.xy, r1.xzzw, c21.yyyy
mul oT2.xy, r1.xyzw, c8.wwww
dp3 r0.x, r0.xyzw, r0.xyzw
rsq r0.x, r0.xxxx
rcp r0.x, r0.xxxx
mul r0.x, r0.xxxx, c18.xxxx
max r0.x, r0.xxxx, c8.xxxx
min r0.x, r0.xxxx, c8.yyyy
mul oT1.x, r0.xxxx, c18.yyyy
dp3 r0.x, v1.xyzw, c5.xyzw
dp3 r0.y, v1.xyzw, c6.xyzw
dp3 r0.z, v1.xyzw, c7.xyzw
dp3 r0.x, r0.xyzw, c2.xyzw
mad r0.x, r0.xxxx, c8.zzzz, c8.zzzz
max r0.x, r0.xxxx, c8.xxxx
min r0.x, r0.xxxx, c8.yyyy
mov r0.yz, c8.xyzw
mul r1.xyz, r0.zzzz, c1.xyzw
mov r2.xyz, c1.xyzw
mad r2.xyz, r2.xyzw, r0.zzzz, c0.xyzw
mad r0.xzw, r0.xxxx, r2.xyyz, r1.xyyz
mul oD0.xyz, r0.xzww, v2.xyzw
dp4 r1.z, v0.xyzw, c11.xyzw
add r0.x, -r1.zzzz, -c17.zzzz
mad r0.x, r0.xxxx, -c17.wwww, r0.yyyy
max r0.x, r0.xxxx, c8.xxxx
min r0.x, r0.xxxx, c8.yyyy
mul r0.xyz, r0.xxxx, c19.xyzw
dp4 r1.x, v0.xyzw, c9.xyzw
dp4 r1.y, v0.xyzw, c10.xyzw
mad r0.xyz, r0.xyzw, v4.xxxx, r1.xyzw
dp4 r0.w, v0.xyzw, c12.xyzw
dp4 oPos.x, r0.xyzw, c13.xyzw
dp4 oPos.y, r0.xyzw, c14.xyzw
dp4 oPos.z, r0.xyzw, c15.xyzw
dp4 oPos.w, r0.xyzw, c16.xyzw
mad r0.xyz, v3.xyxw, c8.yyxw, c8.xxyw
dp3 oT0.x, r0.xyzw, c3.xyww
dp3 oT0.y, r0.xyzw, c4.xyww
mov oD0.w, v2.wwww
mov oT2.z, c20.xxxx
mov oT2.w, v4.xxxx
