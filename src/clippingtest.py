import pygame as pyg
import math

class Vec2:
    def __init__(self, x: float, y: float):
        self.x = x
        self.y = y

    def dot(self, other):
        return self.x*other.x + self.y*other.y
    
    def normalize(self):
        mag = math.sqrt(self.dot(self))
        return Vec2(self.x/mag, self.y/mag)
    
    def __add__(self, other):
        if isinstance(other, Vec2):
            return Vec2(self.y + other.x, self.y + other.y)
        return Vec2(self.x + other, self.y + other)
    
    def __sub__(self, other):
        if isinstance(other, Vec2):
            return Vec2(self.x - other.x, self.y - other.y)
        return Vec2(self.x - other, self.y - other)
    
    def __mul__(self, other):
        if isinstance(other, Vec2):
            return Vec2(self.x * other.x, self.y * other.y)
        return Vec2(self.x * other, self.y + other)
    
    def __repr__(self):
        return f"({self.x}, {self.y})"
    
    def to_tup(self) -> tuple[float, float]:
        return (self.x, self.y)
    
class Line:
    def __init__(self, samplepoint: Vec2, normal: Vec2):
        self.samplepoint = samplepoint
        self.normal = normal
    
    def intersect_line(self, line_start: Vec2, line_end: Vec2) -> Vec2:
        line_slope = line_end - line_start

        t = -self.normal.dot(line_start - self.samplepoint) / self.normal.dot(line_slope)
        return (line_slope*t + line_start)

def clip_tri(tri: list[Vec2], bounds: list[Line]) -> tuple[Vec2, Vec2, Vec2]:
    checkverts = tri
    vertex_at_zero = 0 # tracks the start-vertex of the current loop through the triangle

    i = 1
    while i < 4:
        for j in range(len(bounds)):
                # dot a vector pointing from the plane to the vertex with the plane's normal, to see if it is behind
            if (checkverts[i-1] - bounds[j].samplepoint).dot(bounds[j].normal) <= 0:
                if i-1 != 0 : # Since we already passed over vertex 0, we know it is valid, so we can use it as an anchor
                    pyg.draw.line(screen, (0, 0, 0), checkverts[i-1].to_tup(), checkverts[0].to_tup())
                    checkverts[i-1] = bounds[j].intersect_line(checkverts[0], checkverts[i-1])
                else:
                    if vertex_at_zero != 2: # If we are vertex 0 and are invalid, if we have not checked all three vertices,
                            # try looping through the three again but offset so the *next* element has index 0
                        vertex_at_zero+=1
                        v = checkverts[0]
                        checkverts[0] = checkverts[1]
                        checkverts[1] = checkverts[2]
                        checkverts[2] = v
                        i = 0
                    else:
                        print("triangle outside!")
                        return [Vec2(0, 0) for i in range(3)] # All three vertices are invalid; return a control value
                
        i+=1
    
    return tri

testtris = [#[Vec2(110, 10), Vec2(200, 20), Vec2(190, 100)] # fully inside
            # ,[Vec2(50, 100), Vec2(300, 80), Vec2(250, 150)] # all verts outside; traverses screen
             [Vec2(140, 150), Vec2(220, 150), Vec2(200, 200)] # 1 inside
             ,[Vec2(70, 170), Vec2(140, 170), Vec2(130, 220)]] # 2 inside
            # ,[Vec2(370, 150), Vec2(300, 130), Vec2(250, 200)]] # fully outside
#for i in range()
clipbounds = [Line(Vec2(100, 2), Vec2(1, 0)), Line(Vec2(200, 100), Vec2(-0.95, -math.sqrt(1 - 0.95**2)))]
pointqueue = []

pyg.init()
screen = pyg.display.set_mode((400, 300))
fps = 60
clock = pyg.time.Clock()
screen.fill((255, 255, 255))

running = True

while running:
    for event in pyg.event.get():
        if event.type == pyg.QUIT:
            running = False
        elif event.type == pyg.MOUSEBUTTONUP:
            if event.button == 1:
                mpos = pyg.mouse.get_pos()
                pointqueue.append(Vec2(mpos[0], mpos[1]))
                while len(pointqueue) > 3:
                    pointqueue.pop(0)
                if len(pointqueue) == 3:
                    testtris.append(pointqueue.copy())
                    print(testtris)
            elif event.button == 3:
                pointqueue.clear()

    # Update Functionality
    #screen.fill((255, 255, 255))

    for tri in testtris:
        pyg.draw.polygon(screen, (0, 255, 255), ((tri[0].x, tri[0].y), (tri[1].x, tri[1].y), (tri[2].x, tri[2].y)))

    for tri in testtris:
        clippedtri = clip_tri(tri, clipbounds)
        pyg.draw.polygon(screen, (255, 0, 0), ((clippedtri[0].x, clippedtri[0].y), (clippedtri[1].x, clippedtri[1].y), (clippedtri[2].x, clippedtri[2].y)))

    for bound in clipbounds:
        start: tuple[int, int] = ((bound.samplepoint.dot(bound.normal))/bound.normal.x, 0)
        end: tuple[int, int] = ((bound.samplepoint.dot(bound.normal) - 300*bound.normal.y)/bound.normal.x, 300)
        pyg.draw.line(screen, (0, 0, 0), start, end)

    pyg.display.flip()

    # FPS Tick
    clock.tick(fps)

pyg.quit()