"""Desenho isométrico 2D de peças: caixas, prismas extrudados, cilindros e cotas alinhadas, com faces opacas desenhadas do fundo
para a frente (o que fica atrás some sozinho). Eixos: E1 desce para a direita, E2 sobe para a direita, Z sobe.
Usado por gerar_peca_*.py; a saída é texto `.fct` (uma linha por objeto)."""
import math

C, S = math.cos(math.radians(30)), math.sin(math.radians(30))


def n(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


class Iso:
    def __init__(self, k=1.0, ox=0.0, oy=0.0):
        self.k, self.ox, self.oy = k, ox, oy
        self.out = []
        self.camada_atual = None
        self.camadas = {}

    # ---- projeção
    def p(self, e1, e2, z):
        return (self.ox + (e1 + e2) * C * self.k, self.oy + ((e2 - e1) * S + z) * self.k)

    # ---- saída
    def camada(self, nome, cor):
        if self.camada_atual != nome:
            self.camadas.setdefault(nome, cor)
            self.out.append(f"camada {nome} {self.camadas[nome]}")
            self.camada_atual = nome

    def linha(self, a, b, cor=None):
        self.out.append(f"L {n(a[0])} {n(a[1])} {n(b[0])} {n(b[1])}" + (f" {cor}" if cor else ""))

    def poli(self, pts, fechada=False, cor=None):
        self.out.append("P " + " ".join(f"{n(x)} {n(y)}" for x, y in pts) + (" f" if fechada else "") + (f" {cor}" if cor else ""))

    def preencher(self, laços, cor):
        self.out.append("H solida " + " | ".join(" ".join(f"{n(x)} {n(y)}" for x, y in l) for l in laços) + f" {cor}")

    def hachurar(self, laços, nome="padrao", cor=None):
        self.out.append(f"H {nome} " + " | ".join(" ".join(f"{n(x)} {n(y)}" for x, y in l) for l in laços) + (f" {cor}" if cor else ""))

    def texto(self, x, y, h, txt, giro=0, cor=None):
        self.out.append(f'T {n(x)} {n(y)} {n(h)} {n(giro)} "{txt}"' + (f" {cor}" if cor else ""))

    # ---- faces
    def face(self, pts3, cor_fundo):
        """Uma face plana (lista de pontos (e1,e2,z)) opaca, com contorno."""
        pts = [self.p(*q) for q in pts3]
        self.camada("FACES", "#FFFFFF")
        self.preencher([pts], cor_fundo)
        self.camada("CONTORNO", "#FFFFFF")
        self.poli(pts, True)

    def caixa(self, e1a, e1b, e2a, e2b, za, zb, topo="#F2F2F2", frente="#E2E2E2", esquerda="#CFCFCF"):
        """Só as três faces visíveis: topo, +E1 (frente, à direita) e -E2 (esquerda)."""
        self.face([(e1a, e2a, zb), (e1b, e2a, zb), (e1b, e2b, zb), (e1a, e2b, zb)], topo)
        self.face([(e1b, e2a, za), (e1b, e2b, za), (e1b, e2b, zb), (e1b, e2a, zb)], frente)
        self.face([(e1a, e2a, za), (e1b, e2a, za), (e1b, e2a, zb), (e1a, e2a, zb)], esquerda)

    def prisma(self, perfil, e2a, e2b, base="e1", topo="#F2F2F2", frente="#E2E2E2", esquerda="#CFCFCF", tampa="#CFCFCF", hach=None):
        """Perfil (u, z) extrudado ao longo do outro eixo de e2a a e2b. `base='e1'`: u = E1, extrusão em E2 (tampa visível em E2=e2a,
        a face -E2). `base='e2'`: u = E2, extrusão em E1 (tampa visível em E1=e2b, a face +E1)."""
        if base == "e1":
            pt = lambda u, z, w: (u, w, z)
            tampa_w = e2a
        else:
            pt = lambda u, z, w: (w, u, z)
            tampa_w = e2b
        m = len(perfil)
        for i in range(m):  # lados (arestas do perfil), visíveis se a normal olha para o observador (1,-1,1)
            (u0, z0), (u1, z1) = perfil[i], perfil[(i + 1) % m]
            du, dz = u1 - u0, z1 - z0
            # normal para fora (perfil anti-horário em (u,z)): (dz, -du)
            nu, nz = dz, -du
            v = (nu + nz) if base == "e1" else (nu * -1 + nz) if False else (nu + nz) if base == "e1" else (-nu + nz)
            # base e1: u=E1 -> visível se nu + nz > 0 ; base e2: u=E2 -> visível se -nu + nz > 0
            if v <= 1e-9:
                continue
            cor = topo if abs(nz) > abs(nu) else (frente if (nu > 0) == (base == "e1") else esquerda)
            self.face([pt(u0, z0, e2a), pt(u1, z1, e2a), pt(u1, z1, e2b), pt(u0, z0, e2b)], cor)
        cap = [self.p(*pt(u, z, tampa_w)) for u, z in perfil]
        self.camada("FACES", "#FFFFFF")
        self.preencher([cap], tampa)
        self.camada("CONTORNO", "#FFFFFF")
        self.poli(cap, True)
        if hach:
            self.camada("HACHURA", "#FFFFFF")
            self.hachurar([cap], hach)
        return cap

    # ---- cotas alinhadas em isométrico
    def cota(self, a3, b3, off3, texto, cor="#3B82F6", altura=0.5, ext=0.25):
        """Cota entre dois pontos 3D (e1,e2,z), afastada por `off3` (vetor 3D). As linhas de extensão vão do ponto até a linha de cota."""
        pa, pb = self.p(*a3), self.p(*b3)
        qa = self.p(a3[0] + off3[0], a3[1] + off3[1], a3[2] + off3[2])
        qb = self.p(b3[0] + off3[0], b3[1] + off3[1], b3[2] + off3[2])
        self.camada("COTAS", cor)
        self.linha(pa, qa, cor)
        self.linha(pb, qb, cor)
        self.linha(qa, qb, cor)
        for q, o in ((qa, qb), (qb, qa)):  # setas
            dx, dy = o[0] - q[0], o[1] - q[1]
            d = math.hypot(dx, dy) or 1.0
            ux, uy = dx / d, dy / d
            t = 0.55 * self.k
            self.poli([(q[0] + ux * t + -uy * t * 0.25, q[1] + uy * t + ux * t * 0.25), q, (q[0] + ux * t + uy * t * 0.25, q[1] + uy * t - ux * t * 0.25)], False, cor)
        ang = math.degrees(math.atan2(qb[1] - qa[1], qb[0] - qa[0]))
        if ang > 90 or ang < -90:
            ang += 180
        mx, my = (qa[0] + qb[0]) / 2, (qa[1] + qb[1]) / 2
        rad = math.radians(ang)
        h = altura * self.k
        self.texto(mx - math.cos(rad) * len(texto) * h * 0.3 + math.sin(rad) * h * 0.2, my - math.sin(rad) * len(texto) * h * 0.3 - math.cos(rad) * h * 0.2 + 0.0, h, texto, ang, cor)

    def elipse_pts(self, ce1, ce2, z, r, a0=0.0, a1=360.0, passo=6.0):
        m = max(2, int(abs(a1 - a0) / passo) + 1)
        return [self.p(ce1 + r * math.cos(math.radians(a0 + (a1 - a0) * i / m)), ce2 + r * math.sin(math.radians(a0 + (a1 - a0) * i / m)), z) for i in range(m + 1)]
