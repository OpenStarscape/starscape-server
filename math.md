# Useful Math
Mostly orbital mechanics related 

## Resources
- [Orbital Mechanics 101 video](https://www.youtube.com/watch?v=VGcQhgkXPx0&t=600s)
- [Vis-viva equation](https://en.wikipedia.org/wiki/Vis-viva_equation)
- [sphere of influence](https://en.wikipedia.org/wiki/Sphere_of_influence_(astrodynamics))
- [Orbital state vectors](https://en.wikipedia.org/wiki/Orbital_state_vectors)
- [Orbital elements](https://en.wikipedia.org/wiki/Orbital_elements)
- [Stack Exchange answer on positions from orbits](https://space.stackexchange.com/a/8915)

## Units
These are the standard units assumed throughout the Starscape server and protocol
- Distance: kilometers (km)
- Time: seconds (real-time seconds and in-game seconds are generally assumed to match) (s)
- Mass: metric tons (t)
Other units and constants are derived from these (so our G is 6.67430e-17 instead of …e-11)

## Definitions
- G: gravitational constant (6.67430e-17) (unit: m³⋅kg⁻¹⋅s⁻²)
- v: relative velocity (unit: km/s)
- r: distance between the two bodies (radius?? this would be very incorrect, but r does seem to be the convention) (unit: km)
- rₚ: radius at the periapsis (closest point to gravity well)
- rₐ: radius at the apoapsis (farthest point from gravity well)
- rSOI: radius of the [sphere of influence](https://en.wikipedia.org/wiki/Sphere_of_influence_(astrodynamics)) (unit: km)
- m: mass of the body in question (like a ship or a planet) (unit: t)
- M: mass of the gravity well in question (like a planet or the sun) (unit: t)
- E: energy (unit: t⋅km²⋅s⁻²)
- L: angular momentum (unit:  km²·s⁻¹·t)
- h: specific angular momentum (L/m) (unit:  km²·s⁻¹)
- a: semi-major axis (long radius) of the orbit (unit: km)
- b: semi-minor axis (short radius) of the orbit (unit: km)
- e: eccentricity of the orbit
- θₘ: angle between relative velocity and direction towards the central body
- T: orbital period (time taken to complete an orbit)

## Equations
- E = ½mv² - (GmM)/r
- L = mrv⋅sin(θₘ)
- rₚ + rₐ = 2a _(obvious)_
- rₚrₐ = b² _(less obvious)_
- b = a√(1 - e²)
  - e = √(a² - b²)/a
- v² = GM(2/r - 1/a) _(vis-viva equation)_
  - a = rGM/(2GM - rv²)
- semi-major axis
  - ellipse: a > 0
  - hyperbola: a < 0
  - parabolas: a = ∞ or 1/a = 0 (wtf?)
  - accidentally using the same body for both: a = 0
- eccentricity
  - circle: e = 0
  - ellipse: e > 0, e < 1
  - hyperbola: e > 1
  - parabola: e = 1
- rSOI ≈ a(m/M)^(2/5)
- T = 𝜏√(a³/GM) _(Kepler's Third Law)_

## Laws
- Conservation of angular momentum: in free orbit L stays the same (as does h assuming mass doesn't change)
- Conservation of energy: in free orbit E stays the same
