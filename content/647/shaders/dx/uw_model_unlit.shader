version=vs_1_1
ctab.flags=295168
ctab.constant=fogParams,c11,1,46,type0
ctab.constant=fogPlanes,c10,1,42,type0
ctab.constant=modelViewMatrix,c0,4,2,type1
ctab.constant=projectionMatrix,c4,4,18,type1
ctab.constant=textureMatrix,c8,2,34,type2
ctab.constant=waterPlane,c12,1,50,type0
ctab.string=fogParams,"fogParams"
ctab.padding=171,2
ctab.type=type0,vector,float,1,4,1
ctab.string=fogPlanes,"fogPlanes"
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.padding=171,2
ctab.type=type1,matrix_columns,float,4,4,1
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,1
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=waterPlane,"waterPlane"
ctab.string=target,"vs_1_1"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,1
def c13.xyzw, 0, 1, 0, 0
dcl position0, v0.xyzw
dcl color0, v1.xyzw
dcl texcoord0, v2.xyzw
dp4 r0.z, v0.xyzw, c2.xyzw
add r1.xy, -r0.zzzz, -c10.xzzw
mul r1.xz, r1.xyyw, c10.yyww
dp4 r0.x, v0.xyzw, c0.xyzw
dp4 r0.y, v0.xyzw, c1.xyzw
dp4 r0.w, v0.xyzw, c3.xyzw
dp4 r1.w, c12.xyzw, r0.xyzw
mad r1.y, -r1.wwww, c11.xxxx, c11.yyyy
max r1.xyz, r1.xyzw, c13.xxxx
min r1.xyz, r1.xyzw, c13.yyyy
max oT1.x, r1.yyyy, r1.xxxx
mul r1.xyz, r1.zzzz, c12.xyzw
mad r0.xyz, r1.xyzw, -r1.wwww, r0.xyzw
dp4 oPos.x, r0.xyzw, c4.xyzw
dp4 oPos.y, r0.xyzw, c5.xyzw
dp4 oPos.z, r0.xyzw, c6.xyzw
dp4 oPos.w, r0.xyzw, c7.xyzw
add r0.x, r0.zzzz, c11.zzzz
mul r0.x, r0.xxxx, c11.wwww
max r0.x, r0.xxxx, c13.xxxx
min oFog.xyzw, r0.xxxx, c13.yyyy
mad r0.xyz, v2.xyxw, c13.yyxw, c13.xxyw
dp3 oT0.x, r0.xyzw, c8.xyww
dp3 oT0.y, r0.xyzw, c9.xyww
mov oD0.xyzw, v1.xyzw
