version=vs_1_1
ctab.flags=295168
ctab.constant=ambient,c13,1,54,type0
ctab.constant=antiSunColour,c16,1,66,type0
ctab.constant=antiSunDirection,c17,1,70,type0
ctab.constant=fogParams,c11,1,46,type1
ctab.constant=fogPlanes,c10,1,42,type1
ctab.constant=modelViewMatrix,c0,4,2,type2
ctab.constant=projectionMatrix,c4,4,18,type2
ctab.constant=sunColour,c14,1,58,type0
ctab.constant=sunDirection,c15,1,62,type0
ctab.constant=textureMatrix,c8,2,34,type3
ctab.constant=waterPlane,c12,1,50,type1
ctab.string=ambient,"ambient"
ctab.type=type0,vector,float,1,3,1
ctab.string=antiSunColour,"antiSunColour"
ctab.string=antiSunDirection,"antiSunDirection"
ctab.string=fogParams,"fogParams"
ctab.padding=171,3
ctab.type=type1,vector,float,1,4,1
ctab.string=fogPlanes,"fogPlanes"
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.padding=171,2
ctab.type=type2,matrix_columns,float,4,4,1
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=sunColour,"sunColour"
ctab.string=sunDirection,"sunDirection"
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,2
ctab.type=type3,matrix_columns,float,4,2,1
ctab.string=waterPlane,"waterPlane"
ctab.string=target,"vs_1_1"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,1
def c18.xyzw, 0, 1, 0, 0
dcl position0, v0.xyzw
dcl normal0, v1.xyzw
dcl color0, v2.xyzw
dcl texcoord0, v3.xyzw
dp4 r0.z, v0.xyzw, c2.xyzw
add r1.xy, -r0.zzzz, -c10.xzzw
mul r1.zw, r1.xyxy, c10.xyyw
dp4 r0.x, v0.xyzw, c0.xyzw
dp4 r0.y, v0.xyzw, c1.xyzw
dp4 r0.w, v0.xyzw, c3.xyzw
dp4 r2.x, c12.xyzw, r0.xyzw
mad r1.y, -r2.xxxx, c11.xxxx, c11.yyyy
mul r1.x, r1.yyyy, r1.zzzz
max r1.xyz, r1.xyww, c18.xxxx
min r1.xyz, r1.xyzw, c18.yyyy
max oT1.x, r1.yyyy, r1.xxxx
mul r1.xyz, r1.zzzz, c12.xyzw
mad r0.xyz, r1.xyzw, -r2.xxxx, r0.xyzw
mov r1.x, c0.xxxx
mov r1.y, c1.xxxx
mov r1.z, c2.xxxx
dp3 r1.x, v1.xyzw, r1.xyzw
mov r2.x, c0.yyyy
mov r2.y, c1.yyyy
mov r2.z, c2.yyyy
dp3 r1.y, v1.xyzw, r2.xyzw
mov r2.x, c0.zzzz
mov r2.y, c1.zzzz
mov r2.z, c2.zzzz
dp3 r1.z, v1.xyzw, r2.xyzw
dp3 r2.x, r1.xyzw, c15.xyzw
dp3 r2.y, r1.xyzw, c17.xyzw
max r1.xy, r2.xyzw, c18.xxxx
min r1.xy, r1.xyzw, c18.yyyy
mov r2.xyz, c14.xyzw
mad r1.xzw, r2.xyyz, r1.xxxx, c13.xyyz
mad r1.xyz, c16.xyzw, r1.yyyy, r1.xzww
mul oD0.xyz, r1.xyzw, v2.xyzw
dp4 oPos.x, r0.xyzw, c4.xyzw
dp4 oPos.y, r0.xyzw, c5.xyzw
dp4 oPos.z, r0.xyzw, c6.xyzw
dp4 oPos.w, r0.xyzw, c7.xyzw
add r0.x, r0.zzzz, c11.zzzz
mul r0.x, r0.xxxx, c11.wwww
max r0.x, r0.xxxx, c18.xxxx
min oFog.xyzw, r0.xxxx, c18.yyyy
mad r0.xyz, v3.xyxw, c18.yyxw, c18.xxyw
dp3 oT0.x, r0.xyzw, c8.xyww
dp3 oT0.y, r0.xyzw, c9.xyww
mov oD0.w, v2.wwww
